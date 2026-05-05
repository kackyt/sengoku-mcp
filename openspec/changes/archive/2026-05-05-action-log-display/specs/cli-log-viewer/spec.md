## ADDED Requirements

### Requirement: mode-aware-display
MUST: CLIクライアントは、現在のゲームフェーズ（内政/合戦）に応じて、表示するログのカテゴリを自動的に切り替えなければならない。

#### Scenario: switch-to-domestic-log
- **WHEN** 内政画面が表示されているとき
- **THEN** `Domestic` カテゴリの最新10件のログが表示される

#### Scenario: switch-to-war-log
- **WHEN** 合戦画面が表示されているとき
- **THEN** `War` カテゴリの最新10件のログが表示される

### Requirement: visibility-filter
SHALL: CLIは、`Visibility` が `Public` または `Player` のログのみを表示しなければならない。`Internal` は表示してはならない。

#### Scenario: hide-internal-logs
- **WHEN** ログ表示領域が描画されるとき
- **THEN** `Internal` の Visibility を持つログは一切表示されない（チート防止）

### Requirement: log-display-area
MUST: CLIクライアントは、メイン画面の下部にログを表示するための固定領域を持たなければならない。

#### Scenario: render-log-area
- **WHEN** ゲームのメイン画面が描画されるとき
- **THEN** 画面下部に「Action Log」タイトルの枠があり、ログ行が表示される

#### Scenario: small-terminal-handling
- **WHEN** ターミナルの高さが20行以下のとき
- **THEN** ログ表示領域を非表示にし、マップ表示領域を最大化する

### Requirement: log-layering
SHALL: ログ表示領域は、コマンドメニューや合戦の戦術選択などのポップアップウィンドウの背面に描画されなければならない。

#### Scenario: overlay-priority
- **WHEN** 戦術選択メニューなどのポップアップが表示されているとき
- **THEN** ポップアップがログ領域に重なる場合、ポップアップの内容が優先的に（手前に）表示される
