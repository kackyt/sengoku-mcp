## 1. ドメインとユースケースの実装

- [x] 1.1 他国の情報を集計するためのDTO `OtherCountriesInfoDTO` を `engine/src/application/dto` に定義する。
- [x] 1.2 `InfoUseCase` を `engine/src/application/usecase/info_usecase.rs` に実装する。
    - [x] 1.2.1 自分以外の大名の領地情報を `KuniRepository` から取得するロジックを実装。
    - [x] 1.2.2 取得した情報を米、金、兵、石高、町、忠誠度の項目ごとに集計し、大名ごとにまとめる。
    - [x] 1.2.3 実行時に `GameStateRepository` から現在の状態を取得し、手番の大名のアクションを完了させる（`complete_current_action` 相当）処理を実装。
- [x] 1.3 `engine/src/application/usecase/mod.rs` に `info_usecase` を追加。

## 2. MCP プレゼンテーション層の実装

- [x] 2.1 `mcp-server/src/presentation/handlers.rs` に `handle_get_other_countries_info` ハンドラを追加する。
- [x] 2.2 MCP サーバーのツール定義に `get_other_countries_info` ツールを追加する。
- [x] 2.3 `mcp-server/src/main.rs` 等の構成ルートで `InfoUseCase` を初期化・注入し、ツールと紐付ける。

## 3. テストと検証

- [x] 3.1 `InfoUseCase` の単体テストを作成し、他国の情報が正しく集計されること、アクションが進行すること、アクション権限がない場合にエラーになることを確認する。
- [x] 3.2 MCP経由でツールを実行し、他国の情報が一覧表示されることを確認する。
