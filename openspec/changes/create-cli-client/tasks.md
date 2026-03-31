## 1. Setup and Initialization

- [x] 1.1 `cli` クレートを `cargo new cli` で作成し、ワークスペースの `members` に追加する
- [x] 1.2 `Cargo.toml` に `ratatui`, `crossterm`, `engine`, `infrastructure` を追加する
- [x] 1.3 `main.rs` で crossterm の raw mode と Terminal の初期化処理を実装する
- [x] 1.4 `App` 構造体を定義し、インフラリポジトリとユースケースのDI初期化処理を実装する

## 2. Core Event Loop and State Management

- [x] 2.1 画面状態を表す Enum `ScreenState` を定義する（Title, SelectDaimyo, Domestic, War など）
- [x] 2.2 `ratatui` を用いたメインの `draw` ループフレームワークを実装する
- [x] 2.3 `crossterm` によるキーイベント取得とディスパッチ用イベントループを実装する

## 3. Title and Daimyo Selection UI

- [x] 3.1 タイトル画面のレンダリングを実装する
- [x] 3.2 大名選択画面（一覧表示とカーソル移動）のレンダリングを実装する
- [x] 3.3 大名選択時の `Domestic` 状態への遷移と、選択されたプレイヤー大名のゲーム状態への登録処理を実装する

## 4. Domestic Mode UI and Logic

- [x] 4.1 国内リソース（金、米、兵など）を描画するステータスパネルを実装する
- [x] 4.2 コマンドメニュー（戦争、米売り、米買い、開墾など）のリストUIとカーソル移動処理を実装する
- [/] 4.3 コマンド決定時の数値入力（または固定値適用の）フローを実装する（対象国選択は未実装）
- [x] 4.4 `DomesticUseCase` を呼び出し、各種コマンドのロジック実行と再描画を連携させる

## 5. War Mode UI and Logic

- [ ] 5.1 戦争対象の領地を選択するUIを実装する
- [/] 5.2 戦争モード時の両軍ステータスと、戦術選択（通常、奇襲、火計など）メニューを描画するパネルを実装する（プレースホルダー）
- [/] 5.3 `BattleUseCase` を呼び出し、戦闘の実行結果を描画・反映する処理を実装する（最低限の実装）

## 6. Turn Progression and Messaging

- [/] 6.1 `TurnProgressionUseCase` と連携し、プレイヤー行動終了時やNPCターンの処理をUI上から実行させる（簡略化されている）
- [ ] 6.2 ターン進行中の簡易的なログ・メッセージポップアップ表示を実装する
- [x] 6.3 ターミナル終了時のクリーンアップ処理（raw mode の解除・画面の復元）が正常に機能することを検証する
