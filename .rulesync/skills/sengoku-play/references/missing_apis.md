# 実装済みMCP APIメモ

このファイルには、以前は不足していたが、現在はMCPサーバーに実装された追加APIを参考用にまとめます。
このリストは当該PR時点での実装状態を示すメモです。

## 実装済みAPI

### 1. 実装済み: `get_game_status` - ゲーム状態の取得

**必要な理由:** 現在の `get_my_status` は自分の領地情報しか返さない。ゲームフェーズ（Domestic/Battle/GameOver）、勝者情報、ターンの季節情報を取得するAPIが必要。

**実装場所:** `mcp-server/src/presentation/handlers.rs`

**実装内容:**
```rust
/// ゲーム全体の状態（フェーズ・ターン・季節・勝者）を取得します
#[tool(description = "ゲーム全体の状態（フェーズ・ターン・季節・勝者）を取得します")]
pub async fn get_game_status(&self) -> Result<String, String> {
    let snapshot = self
        .kuni_query_usecase
        .get_ui_snapshot(None, None, None)
        .await
        .map_err(|e| e.to_string())?;

    let phase_str = format!("{:?}", snapshot.phase);
    let winner_str = snapshot
        .winner
        .and_then(|id| snapshot.all_daimyos.iter().find(|d| d.id == id))
        .map(|d| d.name.0.clone())
        .unwrap_or_else(|| "なし".to_string());

    let turn = snapshot.current_turn.unwrap_or(0);
    // TurnNumber::season() を使って季節を取得
    let season = TurnNumber::new(turn).season();

    Ok(format!(
        "フェーズ: {}\nターン: {}\n季節: {:?}\n勝者: {}",
        phase_str, turn, season, winner_str
    ))
}
```

### 2. 実装済み: `get_battle_status` - 進行中の合戦状態の取得

**必要な理由:** 合戦フェーズ中に現在の兵数・士気・優勢/劣勢状況を確認するAPIが必要。`get_my_status` は合戦状態（WarStatus）の詳細を返さない。

**実装場所:** `mcp-server/src/presentation/handlers.rs`

**実装内容:**
```rust
/// 進行中の合戦の状態（兵数・士気・優劣）を取得します
#[tool(description = "進行中の合戦の状態（兵数・士気・優劣）を取得します")]
pub async fn get_battle_status(&self) -> Result<String, String> {
    let snapshot = self
        .kuni_query_usecase
        .get_ui_snapshot(None, None, None)
        .await
        .map_err(|e| e.to_string())?;

    if snapshot.active_battles.is_empty() {
        return Ok("現在進行中の合戦はありません。".to_string());
    }

    let mut result = String::from("進行中の合戦:\n");
    for battle in &snapshot.active_battles {
        let attacker_name = snapshot
            .kuni_names
            .get(&battle.attacker.kuni_id)
            .cloned()
            .unwrap_or_else(|| "不明".to_string());
        let defender_name = snapshot
            .kuni_names
            .get(&battle.defender.kuni_id)
            .cloned()
            .unwrap_or_else(|| "不明".to_string());

        result.push_str(&format!(
            "攻撃: {} vs 防衛: {}\n  攻撃側: 兵={}, 米={}, 士気={}\n  防衛側: 兵={}, 米={}, 士気={}\n  優劣: {:?}\n",
            attacker_name,
            defender_name,
            battle.attacker.hei.to_display().value(),
            battle.attacker.kome.to_display().value(),
            battle.attacker.morale.value(),
            battle.defender.hei.to_display().value(),
            battle.defender.kome.to_display().value(),
            battle.defender.morale.value(),
            battle.advantage,
        ));
    }
    Ok(result)
}
```

### 3. 実装済み: `get_neighbor_info` - 隣接国情報の取得

**必要な理由:** 合戦や輸送の判断のために、指定した国の隣接国（IDと大名名）を一覧表示するAPIが必要。現状は内部的に `KuniQueryUseCase::get_neighbors` が実装されているが、MCPツールとして公開されていない。

**実装場所:** `mcp-server/src/presentation/handlers.rs`

**実装内容:**
```rust
#[derive(Deserialize, JsonSchema)]
pub struct KuniIdParams {
    /// 対象の国ID
    pub kuni_id: u32,
}

/// 指定した国の隣接国一覧を取得します
/// 注意: 隣接国の内部情報（兵数・資源）は返しません。
/// 詳細な敵国情報が必要な場合は get_other_countries_info を使用してください。
#[tool(description = "指定した国の隣接国（攻撃・輸送可能な国）の一覧を取得します。名前・ID・味方/敵のみ返します。")]
pub async fn get_neighbor_info(
    &self,
    Parameters(KuniIdParams { kuni_id }): Parameters<KuniIdParams>,
) -> Result<String, String> {
    let id = KuniId::new(kuni_id);
    let player_id = self.get_player_id().await?;
    let neighbors = self
        .kuni_query_usecase
        .get_neighbors(&id)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = format!("国ID {} の隣接国:\n", kuni_id);
    for n in &neighbors {
        let relation = if n.daimyo_id == player_id { "味方" } else { "敵" };
        // 内部情報（兵数・資源）はチート防止のため返さない
        // 敵国の詳細は get_other_countries_info で取得すること
        result.push_str(&format!("- {} (ID: {}) [{}]\n", n.name.0, n.id.0, relation));
    }
    Ok(result)
}
```

## 当時想定されていた実装手順

1. `handlers.rs` の `McpHandlers` impl ブロックに上記メソッドを追加
2. 必要に応じて `KuniIdParams` などのパラメータ構造体を追加
3. `cargo build` でコンパイルを確認
4. `cargo clippy --all-targets --all-features -- -D warnings` を実行
5. `rulesync generate` でスキルを同期

## 既存APIのギャップ

| 機能 | 現状 | 改善案 |
|---|---|---|
| ゲームフェーズ確認 | `get_my_status` に埋め込まれている | `get_game_status` で独立 |
| 合戦詳細 | なし | `get_battle_status` で追加 |
| 隣接国確認 | なし | `get_neighbor_info` で追加 |
| ターン・季節 | `get_my_status` に含まれる | `get_game_status` で整理 |
