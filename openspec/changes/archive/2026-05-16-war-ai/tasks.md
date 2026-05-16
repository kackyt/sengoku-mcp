## 1. Repositoryインターフェースの拡張

- [x] 1.1 `engine/src/domain/repository/battle_repository.rs` に `find_by_defender(&KuniId) -> Result<Option<WarStatus>>` を追加する
- [x] 1.2 `infrastructure/src/persistence/` の `BattleRepository` 実装に `find_by_defender()` を実装する

## 2. WarDecisionService の新設

- [x] 2.1 `engine/src/domain/service/war_decision_service.rs` を新規作成する
- [x] 2.2 `calculate_attack_threshold(military_bias: f64) -> f64` を実装する
- [x] 2.3 `select_attack_target(kuni: &Kuni, neighbors: &[Kuni], personality: &DaimyoPersonality, rng) -> Option<(KuniId, Amount, Amount)>` を実装する
- [x] 2.4 期待値スコア（勝率ベース）によるターゲット選択ロジックを実装する
- [x] 2.5 出兵判断ロジック（military_bias影響あり）を実装する
- [x] 2.6 `engine/src/domain/service/mod.rs` に `war_decision_service` を追加する
- [x] 2.7 `WarDecisionService` のユニットテストを作成する

## 3. BattleService への戦術AIの追加

- [x] 3.1 `engine/src/domain/service/battle_service.rs` に `decide_tactic_for_attacker(my: &ArmyStatus, enemy: &ArmyStatus, military_bias: f64, rng) -> Tactic` を追加する
- [x] 3.2 攻撃側戦術AIのロジックを実装する
- [x] 3.3 `decide_tactic_for_defender(my: &ArmyStatus, enemy: &ArmyStatus, rng) -> Tactic` を追加する
- [x] 3.4 防衛側戦術AIのロジックを実装する（アンチ選択）
- [x] 3.5 `auto_resolve(attacker_kuni_id, defender_kuni_id, attacker_army, defender_army, rng) -> (WarStatus, u32)` を追加する
- [x] 3.6 自動決着の戦術はNormal/Surprise/Fireを1/3ずつランダムで選択する
- [x] 3.7 `BattleService` の戦術AI・自動決着のユニットテストを作成する

## 4. TurnProgressionUseCase の拡張

- [x] 4.1 `execute_cpu_action()` 内で `WarDecisionService::select_attack_target()` を呼び出し、攻撃可否を判断する処理を追加する
- [x] 4.2 攻撃対象がCPU大名の国の場合、`BattleService::auto_resolve()` を呼んで即座に結果をKuniに反映する処理を実装する
- [x] 4.3 攻撃対象がプレイヤーの国の場合、`dispatch_army()` でリソースを消費し `WarStatus` を `BattleRepository` に保存してターンを終了する処理を実装する
- [x] 4.4 出兵決定時（自動決着・プレイヤー侵攻問わず）に内政アクションログを記録する処理を実装する
- [x] 4.5 `execute_cpu_action` の結合テストを更新・追加する

## 5. BattleUseCase への防衛ユースケースの追加

- [x] 5.1 `engine/src/application/usecase/battle_usecase.rs` に `execute_defense_turn(defender_id: KuniId, defender_tactic: Tactic) -> Result<WarStatus>` を追加する
- [x] 5.2 `find_by_defender()` でWarStatusを取得し、攻撃側CPUの戦術を `decide_tactic_for_attacker()` で決定する処理を実装する
- [x] 5.3 1ターンの合戦計算を行い、勝者確定時の後処理を実装する
- [x] 5.4 `execute_defense_turn` のユニットテストを作成する

## 6. CLIおよびMCPサーバーの拡張

- [x] 6.1 `cli/src/app.rs` でプレイヤーが防衛側の `WarStatus` を検知するロジックを実装する
- [x] 6.2 プレイヤー侵攻通知ログをアクションログに出力する
- [x] 6.3 合戦画面へ自動遷移する処理を実装する
- [x] 6.4 `mcp-server/src/presentation/` に防衛戦術選択MCPツールを追加する
- [x] 6.5 MCPツールの入力バリデーションを実装する

## 7. 追加サービスとライフサイクル管理の実装

- [x] 7.1 `engine/src/domain/service/battle_participation_service.rs` を作成し、プレイヤーの参加判定ロジックを実装する
- [x] 7.2 `engine/src/domain/service/tactic_validation_service.rs` を作成し、戦術選択のバリデーションを実装する
- [x] 7.3 `engine/src/application/usecase/game_lifecycle_usecase.rs` を作成し、ゲームのリセット・初期化ロジックを実装する
- [x] 7.4 `engine/src/application/usecase/tests.rs` に `GameLifecycleUseCase` のテストを追加する
- [x] 7.5 `infrastructure/src/persistence/simulation.rs` を作成し、シミュレーション結果の保存機能を実装する
- [x] 7.6 `infrastructure/src/bin/personality_sim.rs` を更新し、シミュレーション結果の可視化を強化する

## 8. 品質確認

- [x] 8.1 `cargo clippy --all-targets --all-features -- -D warnings` がエラーなく通ることを確認する
- [x] 8.2 `cargo fmt --all -- --check` がエラーなく通ることを確認する
- [x] 8.3 `cargo test` が正常に動作することを確認する
