#!/usr/bin/env python3
"""
sengoku-play お任せレコメンドエンジン (v2)

CpuActionDecisionService の「1単位あたり期待勾配（slope）」ロジックを
Python で再現し、recommend.py の内政レコメンドをCPU同等の精度に引き上げる。

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
      "tyu": <int>,
      "jinko": <int>        // 省略時: max(hei * 3, 300)
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

# --- CPU と合わせた評価係数定数 ---
EVALUATE_HEI_COEF = 50    # 兵力の基本評価係数
EVALUATE_KIN_COEF = 30    # 金の基本評価係数
EVALUATE_KOME_COEF = 20   # 米の基本評価係数
INVESTMENT_HORIZON = 15.0 # 投資の将来収益を何ターン分見込むか


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
    jinko: int = 0           # 省略時は後で補完
    is_mine: bool = True
    daimyo_name: str = ""

    def __post_init__(self):
        # 人口が未指定の場合は兵数の3倍または300の大きい方で推定
        if self.jinko <= 0:
            self.jinko = max(self.hei * 3, 300)
            print(f"Warning: Population (jinko) for {self.name} was not provided. Estimated as {self.jinko}.", file=sys.stderr)


@dataclass
class Action:
    command: str
    kuni_id: int
    kuni_name: str
    amount: int
    score: float
    reason: str


# ---------------------------------------------------------------
# 季節係数 (CPU の turns_to_coef に対応)
# turns: 次のシーズンまでのターン数
# ---------------------------------------------------------------
def turns_to_coef(turns: int) -> float:
    """
    CPU と同じ季節係数テーブル。
    turns=0: 今シーズン(来年) → 60
    turns=1: 次シーズン      → 120
    turns=2                  → 100
    turns=3                  → 80
    それ以外(遠い)            → 0
    """
    table = {0: 60, 1: 120, 2: 100, 3: 80}
    return float(table.get(turns, 0))


def turns_until_season(current_season: int, target_season: int) -> int:
    """
    current_season から target_season まで何ターン後か（0〜3）。
    1年=4ターンの循環。
    """
    return (target_season - current_season) % 4


# ---------------------------------------------------------------
# 「1単位あたり期待勾配 (slope)」の計算
# CPU の calculate_expected_slope に相当する
# ---------------------------------------------------------------
def calculate_slopes(k: Country, season: int, strategy: str) -> dict[str, float]:
    """
    strategy を DaimyoPersonality のバイアスに対応させ、
    各リソース・アクションの勾配を計算する。
    """
    # --- personality バイアス (strategy → CPUの personality.xxx_bias() 相当) ---
    if strategy == "military":
        agriculture_bias = 0.5
        commerce_bias = 0.5
        military_bias = 3.0   # 高い軍事バイアス
    elif strategy == "domestic":
        agriculture_bias = 2.0
        commerce_bias = 2.0
        military_bias = 0.5
    else:  # balanced
        agriculture_bias = 1.0
        commerce_bias = 1.0
        military_bias = 1.0

    # --- 季節係数 ---
    spring_coef = turns_to_coef(turns_until_season(season, 0))
    fall_coef = turns_to_coef(turns_until_season(season, 2))

    kin = k.kin
    kome = k.kome
    hei = k.hei
    jinko = k.jinko
    tyu = k.tyu

    # --- 各リソースの限界効用（持つほど価値減衰） ---
    kin_slope = (EVALUATE_KIN_COEF * commerce_bias) / (1.0 + kin / 100.0)
    kome_slope = (EVALUATE_KOME_COEF * agriculture_bias) / (1.0 + kome / 100.0)

    # 兵力評価（CPU と同じ安全保障ボーナス・抑制ロジック）
    hei_slope = EVALUATE_HEI_COEF * max(military_bias, 1.0)
    if hei < 30 or hei < jinko // 10:
        if jinko > 150:
            hei_slope *= 3.0
        else:
            hei_slope *= 1.5
    if hei >= jinko * 8 // 10:
        hei_slope *= -1.0
    elif hei >= jinko // 2:
        hei_slope *= 0.5

    # 米の過剰補正
    if kome > hei:
        kome_slope *= 0.4
        if kin < 50:
            kome_slope *= 0.5

    # --- 開発系の将来収益勾配 ---
    # 町1単位(100)→ 春に金の32% / 石高1単位(100)→ 秋に米の32%
    machi_unit_slope = 0.32 * kin_slope * (spring_coef / 100.0) * INVESTMENT_HORIZON
    kokudaka_unit_slope = 0.32 * kome_slope * (fall_coef / 100.0) * INVESTMENT_HORIZON
    jinko_unit_slope = (
        0.12 * kin_slope * (spring_coef / 100.0)
        + 0.12 * kome_slope * (fall_coef / 100.0)
    ) * INVESTMENT_HORIZON

    # --- 忠誠度評価 ---
    if tyu < 40:
        tyu_base = 15.0
    elif tyu >= 80:
        tyu_base = 4.0 * 0.01
    elif tyu >= 60:
        tyu_base = 4.0 * 0.03
    elif tyu >= 50:
        tyu_base = 4.0 * 0.1
    else:
        tyu_base = 4.0
    tyu_slope = (tyu_base * 0.3 * spring_coef) + (tyu_base * 0.2 * fall_coef)

    # --- 各アクションの勾配計算（CPU の match atype { ... } に対応） ---
    slopes: dict[str, float] = {}

    # 開墾: コスト10金/単位, 利得5石高
    s = (5.0 * kokudaka_unit_slope) - (10.0 * kin_slope)
    if k.kokudaka < 100:
        s = max(s, 10.0)
    slopes["domestic_develop_land"] = s * agriculture_bias

    # 町作り: コスト10金/単位, 利得5町
    s = (5.0 * machi_unit_slope) - (10.0 * kin_slope)
    if k.machi < 100:
        s = max(s, 10.0)
    slopes["domestic_build_town"] = s * commerce_bias

    # 米売却: コスト1米, 利得0.8金
    slopes["domestic_rice_sell"] = (0.8 * kin_slope) - kome_slope

    # 米購入: コスト1金, 利得0.8米
    slopes["domestic_rice_buy"] = (0.8 * kome_slope) - kin_slope

    # 徴募: コスト0.5金+1人口+0.5忠誠, 利得1兵
    slopes["domestic_recruit"] = (
        hei_slope - (0.5 * kin_slope) - jinko_unit_slope - (0.5 * tyu_slope)
    )

    # 施し: コスト10米, 利得7.5忠誠
    if tyu < 100:
        tyu_gain = 0.75 * tyu_slope
    else:
        tyu_gain = 0.0
    slopes["domestic_give_charity"] = tyu_gain * 10.0 - (10.0 * kome_slope)

    return slopes, dict(
        kin_slope=kin_slope,
        kome_slope=kome_slope,
        hei_slope=hei_slope,
        machi_unit_slope=machi_unit_slope,
        kokudaka_unit_slope=kokudaka_unit_slope,
        tyu_slope=tyu_slope,
    )


# ---------------------------------------------------------------
# 投入量計算（CPU の get_max_affordable + rate 0.3〜0.7 に対応）
# プレイヤー向けに「効果的な量」を算出する
# ---------------------------------------------------------------
def get_recommended_amount(k: Country, command: str) -> int:
    """
    CPU は金の 30%〜70% をランダムで投入する。
    プレイヤー向けには 50% 相当（中間値）を推奨量として返す。
    ただし上限・実用下限を考慮する。
    """
    if command == "domestic_develop_land":
        amt = min(k.kin // 2, 200)
    elif command == "domestic_build_town":
        amt = min(k.kin // 2, 100)
    elif command == "domestic_rice_sell":
        # 兵糧 hei×2 確保後の余剰米の50%
        need = k.hei * 2
        surplus = max(0, k.kome - need)
        amt = min(surplus // 2, 500)
    elif command == "domestic_rice_buy":
        amt = min(k.kin // 2, 200)
    elif command == "domestic_recruit":
        # 目標兵数との差分。金は 1人あたり0.5金 (== kin*2 が雇用可能人数)
        # ドメイン制約: 2 * 新規兵数 + 現在兵数 <= 人口
        affordable = k.kin * 2
        max_by_pop = max(0, (k.jinko - k.hei) // 2)
        amt = min(affordable, max_by_pop, 200)
    elif command == "domestic_give_charity":
        # 忠誠度を70まで上げるのに必要な米量で上限
        needed_tyu = max(0, 70 - k.tyu)
        needed_kome = max(1, needed_tyu * 4 // 3)
        amt = min(k.kome // 2, needed_kome, 100)
    else:
        amt = 0

    return max(0, amt)


# ---------------------------------------------------------------
# 攻撃機会の評価（strategy に基づく）
# ---------------------------------------------------------------
def evaluate_attack_opportunity(
    k: Country,
    enemy_countries: list,
    neighbor_ids: list[int],
    strategy: str,
) -> Optional[tuple[float, int, int, int, str]]:
    """
    攻撃推奨を評価する。
    Returns: (score, target_kuni_id, hei_send, kome_send, reason) or None
    """
    if strategy == "domestic":
        return None

    # 隣接している敵を絞り込む
    neighbor_enemies = [e for e in enemy_countries if e.id in neighbor_ids]
    if not neighbor_enemies:
        return None

    # 出陣に必要な最低兵糧（出兵予定の兵数 × 3）
    hei_send = int(k.hei * 0.7)
    kome_send = hei_send * 3

    if k.kome < kome_send:
        return None
    if k.hei < 20:
        return None

    # military は攻撃倍率を下げる（1.2倍で攻撃推奨）
    required_ratio = 1.5 if strategy == "balanced" else 1.2

    best = None
    for enemy in neighbor_enemies:
        ratio = k.hei / max(enemy.hei, 1)
        if ratio < required_ratio:
            continue

        # 忠誠度が低い敵は占領後の維持が楽 → ボーナス
        tyu_bonus = max(0, 60 - enemy.tyu) * 0.5
        score = (ratio - required_ratio) * 30.0 + tyu_bonus

        if strategy == "military":
            score += 30.0  # 武力重視ボーナス

        reason = (
            f"{enemy.name} への侵攻を推奨 "
            f"（敵兵{enemy.hei}、自軍{k.hei}、兵力比{ratio:.1f}倍）"
        )
        if best is None or score > best[0]:
            best = (score, enemy.id, hei_send, kome_send, reason)

    return best


# ---------------------------------------------------------------
# メインのレコメンド処理
# ---------------------------------------------------------------
def recommend(data: dict) -> list[Action]:
    """全ての自国に対してアクションをレコメンドする"""
    strategy = data.get("strategy", "balanced")
    season = data.get("season", 0)
    neighbor_map: dict[str, list[int]] = data.get("neighbor_map", {})

    my_countries = [
        Country(
            id=c["id"],
            name=c["name"],
            kin=c.get("kin", 0),
            kome=c.get("kome", 0),
            hei=c.get("hei", 0),
            kokudaka=c.get("kokudaka", 0),
            machi=c.get("machi", 0),
            tyu=c.get("tyu", 80),
            jinko=c.get("jinko", 0),
        )
        for c in data.get("my_countries", [])
    ]
    enemy_countries = [
        Country(
            id=c["kuni_id"],
            name=c["kuni_name"],
            kin=c.get("kin", 0),
            kome=c.get("kome", 0),
            hei=c.get("hei", 0),
            kokudaka=c.get("kokudaka", 0),
            machi=c.get("towns", 0),
            tyu=c.get("tyu", 70),
            is_mine=False,
            daimyo_name=c.get("daimyo_name", ""),
        )
        for c in data.get("enemy_countries", [])
    ]

    actions: list[Action] = []

    for k in my_countries:
        neighbor_ids = neighbor_map.get(str(k.id), [])

        # 勾配計算
        slopes, debug_slopes = calculate_slopes(k, season, strategy)

        # 各アクションの候補リスト
        candidates: list[tuple[float, str, int, str]] = []

        for cmd, slope in slopes.items():
            if slope <= 0.0:
                continue
            amount = get_recommended_amount(k, cmd)
            if amount <= 0:
                continue

            # コマンドごとに理由文を作成
            if cmd == "domestic_recruit":
                reason = (
                    f"兵力強化（現在 {k.hei}、勾配 {slope:.1f}）"
                )
            elif cmd == "domestic_develop_land":
                reason = f"石高 {k.kokudaka} → 収入強化（勾配 {slope:.1f}）"
            elif cmd == "domestic_build_town":
                reason = f"町 {k.machi} → 金収入強化（勾配 {slope:.1f}）"
            elif cmd == "domestic_rice_sell":
                reason = f"余剰米を金に換換（勾配 {slope:.1f}）"
            elif cmd == "domestic_rice_buy":
                reason = f"米不足を補填（勾配 {slope:.1f}）"
            elif cmd == "domestic_give_charity":
                reason = f"忠誠度 {k.tyu} → 反乱防止（勾配 {slope:.1f}）"
            else:
                reason = f"勾配 {slope:.1f}"

            candidates.append((slope, cmd, amount, reason))

        # 攻撃判断
        attack = evaluate_attack_opportunity(k, enemy_countries, neighbor_ids, strategy)
        if attack:
            atk_score, target_id, hei_send, kome_send, atk_reason = attack
            # 攻撃は 50 ボーナスで最上位になりやすくする
            candidates.append((atk_score + 50, "battle_start_war", target_id, atk_reason))

        # スコア降順ソート → 上位3件
        candidates.sort(key=lambda x: -x[0])

        for i, (score, cmd, amount, reason) in enumerate(candidates[:3]):
            priority = i + 1
            if cmd == "battle_start_war":
                hei_s = int(k.hei * 0.7)
                kome_s = hei_s * 3
                display = (
                    f"[優先度{priority}] {k.name}(ID:{k.id}) → {cmd}\n"
                    f"  攻撃先: 国ID {amount}, 出陣兵: {hei_s}, 兵糧: {kome_s}\n"
                    f"  理由: {reason}"
                )
            else:
                display = (
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
                    reason=display,
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

    print("=== レコメンド結果 ===")
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
    for a in actions:
        if a.kuni_id not in seen_kunis:
            seen_kunis.append(a.kuni_id)

    for kuni_id in seen_kunis:
        kuni_actions = [a for a in actions if a.kuni_id == kuni_id]
        kuni_name = kuni_actions[0].kuni_name
        print(f"--- {kuni_name} (ID:{kuni_id}) ---")
        for a in kuni_actions:
            print(a.reason)
        print()

    print("※ 国数が増えると各国ごとにコマンドを実行する必要があります。")


if __name__ == "__main__":
    main()
