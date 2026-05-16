## ADDED Requirements

### Requirement: ゲーム状態の初期化とリセット
システムは、ゲームの開始時やリセット時に、すべてのリポジトリを適切な順序でクリアし、マスターデータから初期状態を再構築しなければならない (MUST)。

#### Scenario: ゲームのリセット実行
- **WHEN** `GameLifecycleUseCase::reset_game()` が呼び出された時
- **THEN** システムは以下の順序でデータをクリアしなければならない：
    1. ゲーム状態（GameState）
    2. イベントディスパッチャ（EventDispatcher）
    3. 合戦状態（BattleRepository）
    4. アクションログ（ActionLogRepository）
    5. 国情報（KuniRepository）
    6. 大名情報（DaimyoRepository）
- **AND** クリア後、マスターデータリポジトリから最新のデータをロードし、各リポジトリに保存しなければならない。

#### Scenario: 初期データの整合性
- **WHEN** ゲームが初期化された時
- **THEN** 国情報、大名情報、および隣接関係マップがマスターデータの内容と完全に一致しなければならない。
