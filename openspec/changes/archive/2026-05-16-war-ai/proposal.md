## Why

内政AIは現時点で内政行動（開墾・町造り・徴募など）しか選択できず、`CpuActionDecision::Battle` は型として定義されているが候補リストに含まれていないため絶対に選ばれない。戦国シミュレーションとして合戦は中核機能であり、CPUが自律的に出兵判断・開戦・自動決着を行うロジックが必要である。また、ゲーム全体のライフサイクル管理や、AI挙動を検証するためのシミュレーションツールの強化も求められている。

## What Changes

- `CpuActionDecisionService` に出兵判断ロジックを追加し、`Battle` を選択肢に組み込む
- `WarDecisionService`（新設）が隣接国情報をもとに出兵可否・ターゲット・出兵量を決定する
- `BattleParticipationService`（新設）により、プレイヤーが攻撃・防衛・不参加のどの立場にあるかを判定可能にする
- `TacticValidationService`（新設）により、戦術選択の妥当性を検証する
- `GameLifecycleUseCase`（新設）を導入し、ゲームのリセットやマスターデータからの初期化を統合管理する
- CPU同士の合戦を自動決着させる `auto_resolve` ロジックを `BattleService` に追加する
- CPUが攻撃を選択した際、内政アクションログに「○○が□□へ出陣！」と記録する
- CPUがプレイヤーの国に攻め込んだ時、`WarStatus` を保存し、CLI側でモーダルを表示して合戦モードへ移行する
- `BattleRepository` に `find_by_defender()` を追加し、防衛側からの戦況照会を可能にする
- `execute_defense_turn()` を `BattleUseCase` に追加し、プレイヤーが防衛側として戦術を選択できるようにする
- 攻撃側・防衛側それぞれの戦略的戦術選択AIを `BattleService` に実装する（確率ベース）
- `personality_sim` ツールの強化：シミュレーション結果の保存機能を追加し、最終的な勢力図を可視化する

## Capabilities

### New Capabilities

- `war-decision`: CPUが隣接国の兵力を比較し、出兵可否・ターゲット・出兵量を判断するロジック
- `cpu-auto-battle`: CPU同士の合戦を最大10ターンで自動決着させるロジック。決着なしは攻撃側退却（防衛成功）
- `cpu-defense-turn`: CPUが攻撃側または防衛側として、戦況を読んで確率的に戦術を選択するロジック
- `player-defense-mode`: CPUに攻め込まれたプレイヤーが防衛側として戦術を選択できるモード。CLIでのモーダル通知 and 自動遷移を含む
- `game-lifecycle-management`: ゲームのリセット、初期データのロードなどの管理機能
- `simulation-analytics`: AI挙動のシミュレーション実行と、その結果の永続化・分析機能

### Modified Capabilities

- `execute-battle`: `BattleRepository` に `find_by_defender()` を追加。防衛側プレイヤーの戦況照会に対応する
- `progress-turn`: CPUの自動行動に `Battle` 選択が加わり、相手がCPUなら自動決着、プレイヤーなら合戦フェーズ移行となる
- `cli-ui-rendering`: 合戦ログやプレイヤーへの侵攻通知をリッチに表示するUIレンダラーの拡張

## Impact

- `engine/src/domain/service/cpu_action_decision_service.rs`: `decide()` に `Battle` を候補追加。隣接国情報（`Vec<Kuni>`）を引数に追加
- `engine/src/domain/service/battle_service.rs`: `auto_resolve()`・`decide_tactic_for_attacker()`・`decide_tactic_for_defender()` を追加
- `engine/src/domain/service/battle_participation_service.rs`: プレイヤーの参加形態を判定するロジック
- `engine/src/domain/service/tactic_validation_service.rs`: 戦術の整合性チェック
- `engine/src/domain/repository/battle_repository.rs`: `find_by_defender()` を追加
- `engine/src/application/usecase/battle_usecase.rs`: `execute_defense_turn()` を追加
- `engine/src/application/usecase/game_lifecycle_usecase.rs`: ゲームのリセット・初期化ロジックの統合
- `engine/src/application/usecase/turn_progression_usecase.rs`: `execute_cpu_action()` 内でBattle選択時の分岐を実装（CPU vs CPU / CPU vs Player）およびログ記録
- `infrastructure/src/persistence/simulation.rs`: シミュレーション結果の永続化
- `cli/src/`: プレイヤーへの侵攻を検知してモーダルを表示し、合戦画面へ遷移するロジックの実装。アクションログの表示強化
- `mcp-server/src/presentation/`: 防衛側戦術選択MCPツールを追加（または既存ツールを拡張）
