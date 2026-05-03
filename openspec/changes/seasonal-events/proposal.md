## Why

現在の実装では季節イベントのロジックが `TurnService` に直接記述されており、拡張性が低いです。また、ユーザーが要望している「洪水」や「反乱」といったイベントが実装されていません。季節イベントはゲームのランダム性を高め、戦略的な深みを与える重要な要素であるため、これらを整理し、不足しているイベントを追加する必要があります。

## What Changes

- `SeasonalEventService` を新規作成し、季節イベントのロジックを `TurnService` から分離・移譲します。
- 「洪水」イベントを追加します（夏季に発生し、大幅な被害をもたらす）。
- 「疫病」イベントを修正します（季節を問わず発生、広範な被害）。
- 「反乱」イベントを正式に追加します（忠誠度50未満で発生、甚大なリソース被害）。
- 「資源生成」のタイミングと計算式を修正します（春は金、秋は米）。
- 各イベントの結果をドメインイベントまたは専用のデータ構造として定義し、UIに通知できるようにします。
- `Kuni` モデルにイベントを適用するためのメソッドを追加、または整理します。

## Capabilities

### New Capabilities
- `seasonal-events`: 季節ごとの資源増加、人口変動、およびランダムな災害（疫病、洪水、飢饉、反乱）の処理機能。

### Modified Capabilities
- `progress-turn`: ターン終了時の季節イベント処理の呼び出しと、結果のイベント通知。

## Impact

- `engine/src/domain/service/seasonal_event_service.rs` の新規作成。
- `engine/src/domain/service/turn_service.rs` のリファクタリング。
- `engine/src/application/dto/` または `domain/model/event.rs` へのイベント定義。
- `engine/src/domain/model/kuni.rs` へのロジック追加。
