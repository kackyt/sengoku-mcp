---
name: sengoku-mcp-ai
description: sengoku-mcpのAI挙動分析、評価関数の最適化、および特定大名を操作した対話的なデバッグ・テストを行うためのスキルです。AIが不自然な行動を取った場合や、特定シナリオでのバランス調整が必要な場合に使用します。
---

# Sengoku MCP AI Optimization & Debugging Guide

このスキルは、sengoku-mcpのAI（`CpuActionDecisionService`）の意思決定プロセスを分析・改善し、また開発者が任意の大名を操作して挙動をテストするための手順を提供します。

## 1. AI行動分析ワークフロー

AIの行動が戦略的に不自然な場合（例：兵士の過剰な解雇）、以下の手順で分析・修正します。

### ステップ1: 状態の定量的把握
`mcp_sengoku_get_other_countries_info` を実行し、対象大名の以下の比率を確認します。
- `兵士数 / 石高`: 軍備の充実度
- `金 / 兵士数`: 維持能力
- `忠誠度`: 徴募や輸送への影響

### ステップ2: 評価勾配（Slope）の検証
`engine/src/domain/service/cpu_action_decision_service.rs` の `calculate_expected_slope` を参照し、各アクションの評価勾配が `evaluate_score`（最終的な国力スコア）の偏微分として正しいか確認します。
- **チェックポイント**: `EVALUATE_HEI_COEF`（兵士の価値）が、将来の収入期待値（`jinko_slope` や `tyu_slope`）に負けていないか。

### ステップ3: パラメータ調整
以下の定数を調整し、AIの行動原理を修正します。
- `EVALUATE_HEI_COEF`: 兵士の価値を上げると、解雇が減り雇用が増えます。
- `turns_to_coef`: 収穫（秋）や収入（春）が近いほど、その資源に関わるアクションの評価が高まります。

## 2. 対話型デバッグ・コマンド実行

AIのロジックを検証したり、特定の戦術を試すために、特定の大名を直接操作します。

### 手順
1. **大名の乗っ取り**: `mcp_sengoku_select_daimyo` で操作したい大名のIDを指定します。
2. **現状確認**: `mcp_sengoku_get_my_status` で、その大名の資源、領土、手番を確認します。
3. **対話的コマンド実行**:
   - `mcp_sengoku_domestic_recruit`: 兵士を雇用し、金と人口の消費バランスを確認。
   - `mcp_sengoku_battle_start_war`: 隣国へ合戦を仕掛け、勝敗予測や被害を確認。
4. **AIへの委譲**: `mcp_sengoku_domestic_auto_action` を実行し、AIが同じ状況でどのコマンドを選択するかを観察・比較します。

## 3. トラブルシューティング
- **兵士が減り続ける**: `EVALUATE_HEI_COEF` を 1000 以上に設定することを検討してください。
- **内政を全くしない**: `EVALUATE_KIN_COEF` や `EVALUATE_KOME_COEF` が相対的に低すぎる可能性があります。
- **季節イベントが起きない**: `TurnProgressionUseCase` のターン進行ロジックを確認してください。
