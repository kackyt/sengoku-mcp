#!/usr/bin/env python3
"""
sengoku-play お任せレコメンドエンジン

使い方:
  python recommend.py <input_json_file>

input JSON の構造:
{
  "strategy": "balanced" | "military" | "domestic",
  "season": 0-3,           // 0=春, 1=夏, 2=秋, 3=冬
  "turn": <int>,
  "my_countries": [
    {
      "id": <int>,
      "name": "<str>",
      "kin": <int>,
      "kome": <int>,
      "hei": <int>,
      "kokudaka": <int>,
      "machi": <int>,
      "tyu": <int>
    }, ...
  ],
  "enemy_countries": [      // get_other_countries_info の結果（省略可）
    {
      "kuni_id": <int>,
      "kuni_name": "<str>",
      "daimyo_name": "<str>",
      "kin": <int>,
      "kome": <int>,
      "hei": <int>,
      "kokudaka": <int>,
      "towns": <int>,
      "tyu": <int>
    }, ...
  ],
  "neighbor_map": {         // 各自国IDの隣接国ID一覧（省略可）
    "<kuni_id>": [<neighbor_id>, ...]
  }
}
"""

import json
import sys
from dataclasses import dataclass, field
from typing import Optional


SEASON_NAMES = ["春", "夏", "秋", "冬"]


@dataclass
class Country:
    id: int
    name: str
    kin: int
    kome: int
    hei: int
    kokudaka: int
    machi: int
    tyu: int
    is_mine: bool = True
    daimyo_name: str = ""


@dataclass
class Action:
    command: str
    kuni_id: int
    kuni_name: str
    amount: int
    score: float
    reason: str


def compute_hei_target(k: Country, strategy: str) -> int:
    """戦略と石高から目標兵数を計算する"""
    base = k.kokudaka // 2
    if strategy == "military":
        return int(base * 1.5)
    elif strategy == "domestic":
        return int(base * 0.5)
    return base


def score_recruit(k: Country, strategy: str, season: int) -> tuple[float, int, str]:
    """
    兵を雇うアクションのスコアと推奨量を計算する。
    戦略が military なら優先度が上がる。
    冬は米消費が増えるため注意。
    """
    target = compute_hei_target(k, strategy)
    deficit = max(0, target - k.hei)
    if deficit == 0:
        return 0.0, 0, "兵数は目標値を達成しています"

    # 雇用に必要な金の推定（1兵=10金程度）
    affordable = k.kin // 10
    amount = min(deficit, affordable, 500)
    if amount <= 0:
        return 0.0, 0, "金が不足しています"

    score = 30.0
    if strategy == "military":
        score += 40.0
    elif strategy == "domestic":
        score -= 15.0

    # 忠誠度が低いと反乱リスク上昇 → 兵は多めに
    if k.tyu < 50:
        score += 15.0

    # 秋・冬は攻撃シーズン準備
    if season in (2, 3):
        score += 10.0

    reason = f"目標兵数 {target} に対し現在 {k.hei}（不足 {deficit}）"
    return score, amount, reason


def score_develop_land(k: Country, strategy: str, season: int) -> tuple[float, int, str]:
    """開墾スコアと推奨量。春に特に有効。内政重視なら優先。"""
    if k.kin < 50:
        return 0.0, 0, "金が不足しています"

    amount = min(k.kin // 3, 200)
    score = 25.0
    if strategy == "domestic":
        score += 35.0
    elif strategy == "military":
        score -= 10.0
    if season == 0:  # 春は内政シーズン
        score += 20.0

    reason = f"石高 {k.kokudaka} → 収入強化（春推奨）"
    return score, amount, reason


def score_build_town(k: Country, strategy: str, season: int) -> tuple[float, int, str]:
    """町作りスコアと推奨量。金収入の安定化に有効。"""
    if k.kin < 50:
        return 0.0, 0, "金が不足しています"

    amount = min(k.kin // 3, 100)
    score = 20.0
    if strategy == "domestic":
        score += 30.0
    elif strategy == "military":
        score -= 10.0
    if k.machi < 10:
        score += 15.0  # 町が少ない = 投資効果大

    reason = f"現在の町数 {k.machi}（少ないほど効果大）"
    return score, amount, reason


def score_give_charity(k: Country) -> tuple[float, int, str]:
    """施しスコア。忠誠度 60 未満は優先。"""
    if k.tyu >= 70:
        return 0.0, 0, "忠誠度は十分です"
    if k.kome < 20 and k.kin < 20:
        return 0.0, 0, "資源が不足しています"

    amount = min(k.kin // 2, 100)
    deficit = 70 - k.tyu
    score = deficit * 1.5
    reason = f"忠誠度 {k.tyu}（目標 70 以上、低いと反乱リスク）"
    return score, amount, reason


def score_sell_rice(k: Country, season: int) -> tuple[float, int, str]:
    """米売却スコア。金が少なく米が余っているとき有効。秋は大量売却チャンス。"""
    hei_kome_need = k.hei * 2  # 兵の兵糧として hei×2 を確保
    surplus = k.kome - hei_kome_need
    if surplus <= 0 or k.kin >= k.kokudaka * 2:
        return 0.0, 0, "売却可能な余剰米がありません"

    amount = min(surplus // 2, 500)
    score = 10.0
    if season == 2:  # 秋は収穫後で米が多い
        score += 20.0
    if k.kin < k.kokudaka:
        score += 25.0  # 金が少ない

    reason = f"余剰米 {surplus}（兵糧 {hei_kome_need} 確保後）"
    return score, amount, reason


def evaluate_attack_opportunity(
    k: Country,
    enemy_countries: list[Country],
    neighbor_ids: list[int],
    strategy: str,
) -> Optional[tuple[float, int, int, str]]:
    """
    攻撃推奨を評価する。
    戦術情報（兵数）は get_other_countries_info で得た公開情報のみ使用。
    Returns: (score, target_kuni_id, hei_to_send, reason) or None
    """
    if strategy == "domestic":
        return None  # 内政重視は攻撃しない

    # 隣接している敵国を絞り込む
    neighbor_enemies = [e for e in enemy_countries if e.id in neighbor_ids]
    if not neighbor_enemies:
        return None

    # 攻撃判断: 自軍兵数が敵の 1.5 倍以上、かつ米備蓄が十分
    hei_kome_min = k.hei * 3  # 出陣に必要な兵糧の最低ライン
    if k.kome < hei_kome_min:
        return None

    best: Optional[tuple[float, int, int, str]] = None
    for enemy in neighbor_enemies:
        ratio = k.hei / max(enemy.hei, 1)
        if ratio < 1.5:
            continue
        # 忠誠度が低い敵は占領後の維持が楽
        tyu_bonus = max(0, 60 - enemy.tyu)
        score = (ratio - 1.5) * 30.0 + tyu_bonus * 0.5
        if strategy == "military":
            score += 30.0

        hei_send = int(k.hei * 0.7)
        kome_send = hei_send * 3
        reason = (
            f"{enemy.name} への侵攻を推奨 "
            f"（敵兵 {enemy.hei}、自軍兵 {k.hei}、兵力比 {ratio:.1f}倍）"
        )
        if best is None or score > best[0]:
            best = (score, enemy.id, hei_send, reason)

    return best


def recommend(data: dict) -> list[Action]:
    """全ての自国に対してアクションをレコメンドする"""
    strategy = data.get("strategy", "balanced")
    season = data.get("season", 0)
    turn = data.get("turn", 1)
    neighbor_map: dict[str, list[int]] = data.get("neighbor_map", {})

    my_countries = [Country(**{**c, "is_mine": True}) for c in data.get("my_countries", [])]
    enemy_countries = [
        Country(
            id=c["kuni_id"],
            name=c["kuni_name"],
            kin=c.get("kin", 0),
            kome=c.get("kome", 0),
            hei=c.get("hei", 0),
            kokudaka=c.get("kokudaka", 0),
            machi=c.get("towns", 0),
            tyu=c.get("tyu", 0),
            is_mine=False,
            daimyo_name=c.get("daimyo_name", ""),
        )
        for c in data.get("enemy_countries", [])
    ]

    actions: list[Action] = []

    for k in my_countries:
        neighbor_ids = neighbor_map.get(str(k.id), [])

        # 各アクションをスコア計算
        candidates: list[tuple[float, str, int, str]] = []  # (score, cmd, amount, reason)

        # 施し（忠誠度回復）
        s, amt, r = score_give_charity(k)
        if s > 0:
            candidates.append((s, "domestic_give_charity", amt, r))

        # 米売却
        s, amt, r = score_sell_rice(k, season)
        if s > 0:
            candidates.append((s, "domestic_rice_sell", amt, r))

        # 徴募
        s, amt, r = score_recruit(k, strategy, season)
        if s > 0:
            candidates.append((s, "domestic_recruit", amt, r))

        # 開墾
        s, amt, r = score_develop_land(k, strategy, season)
        if s > 0:
            candidates.append((s, "domestic_develop_land", amt, r))

        # 町作り
        s, amt, r = score_build_town(k, strategy, season)
        if s > 0:
            candidates.append((s, "domestic_build_town", amt, r))

        # 攻撃判断
        attack = evaluate_attack_opportunity(k, enemy_countries, neighbor_ids, strategy)
        if attack:
            atk_score, target_id, hei_send, atk_reason = attack
            # 攻撃は特殊: amount に target_kuni_id を入れる（スクリプト出力で明示）
            candidates.append((atk_score + 50, "battle_start_war", target_id, atk_reason))

        # スコア順にソート
        candidates.sort(key=lambda x: -x[0])

        for i, (score, cmd, amount, reason) in enumerate(candidates[:3]):
            priority = i + 1
            if cmd == "battle_start_war":
                hei_s = int(k.hei * 0.7)
                kome_s = hei_s * 3
                display_reason = (
                    f"[優先度{priority}] {k.name}(ID:{k.id}) → {cmd}\n"
                    f"  攻撃先: 国ID {amount}, 出陣兵: {hei_s}, 兵糧: {kome_s}\n"
                    f"  理由: {reason}"
                )
            else:
                display_reason = (
                    f"[優先度{priority}] {k.name}(ID:{k.id}) → {cmd} 量:{amount}\n"
                    f"  理由: {reason}"
                )
            actions.append(
                Action(
                    command=cmd,
                    kuni_id=k.id,
                    kuni_name=k.name,
                    amount=amount,
                    score=score,
                    reason=display_reason,
                )
            )

    return actions


def main():
    if len(sys.argv) < 2:
        print("使い方: python recommend.py <input_json_file>")
        print("  または: python recommend.py - (標準入力から読み込み)")
        sys.exit(1)

    src = sys.argv[1]
    if src == "-":
        data = json.load(sys.stdin)
    else:
        with open(src, encoding="utf-8") as f:
            data = json.load(f)

    strategy = data.get("strategy", "balanced")
    season = data.get("season", 0)
    turn = data.get("turn", 1)
    my_count = len(data.get("my_countries", []))
    enemy_count = len(data.get("enemy_countries", []))

    print(f"=== レコメンド結果 ===")
    print(f"ターン: {turn}  季節: {SEASON_NAMES[season]}  作戦: {strategy}")
    print(f"自国数: {my_count}  敵情報取得数: {enemy_count}")
    if enemy_count == 0:
        print("  ※ 他国情報なし。get_other_countries_info を実行すると精度が向上します。")
    print()

    actions = recommend(data)
    if not actions:
        print("現在推奨できるアクションはありません（リソース不足または目標達成済み）。")
        return

    # 国ごとにグループ表示
    seen_kunis: list[int] = []
    kuni_order: list[int] = []
    for a in actions:
        if a.kuni_id not in seen_kunis:
            seen_kunis.append(a.kuni_id)
            kuni_order.append(a.kuni_id)

    for kuni_id in kuni_order:
        kuni_actions = [a for a in actions if a.kuni_id == kuni_id]
        kuni_name = kuni_actions[0].kuni_name
        print(f"--- {kuni_name} (ID:{kuni_id}) ---")
        for a in kuni_actions:
            print(a.reason)
        print()

    print("※ 国数が増えると各国ごとにコマンドを実行する必要があります。")


if __name__ == "__main__":
    main()
