## ADDED Requirements

### Requirement: layout-with-action-log
MUST: メインのゲーム画面に、行動ログを表示するための領域が追加されなければならない。

#### Scenario: show-log-area
- **WHEN** ゲームのメイン画面が表示されているとき
- **THEN** 画面下部に「Action Log」というタイトルを持つ枠が表示され、その中にログが表示される

### Requirement: window-size-adaptation
SHALL: 画面サイズが小さい場合、ログ表示領域は自動的に縮小または非表示になり、マップ表示を妨げないようにしなければならない。

#### Scenario: responsive-layout
- **WHEN** ターミナルの高さが一定以下（20行以下）になったとき
- **THEN** ログ表示領域を非表示にし、マップ表示領域を最大化する
