# Change: ゲーム進行処理（ターンループと行動順序）の作成

## Why
現在、`progress-turn`の仕様には季節イベントの処理（人口増加、資源生成、災害）のみが定義されており、ゲームの根幹である「各大名が順番に行動するターン制システム」が欠如しています。プレイヤーとCPUが大名の順にコマンドを実行する仕組みを導入し、ゲームを進行できるようにする必要があります。

## What Changes
- ターンの開始時に大名の行動順序をランダムに決定する仕組みの追加
- 大名が順番に自動で行動（全大名をCPU操作として扱い、戦争や内政を自動実行）する機能
- 各行動やターン進行の際にゲーム進行状況を通知するイベント（Event）送信機能の追加
- これらの行動が完了したのちにターン（季節）を進めるロジックの追加
- 現在の実装（`TurnService`の`process_season`）との結合と再編

## Impact
- Affected specs: `progress-turn`
- Affected code: `engine/src/application/usecase/turn_progression_usecase.rs` (新規), `engine/src/domain/model/game_state.rs` (新規), `engine/src/domain/service/turn_service.rs`
