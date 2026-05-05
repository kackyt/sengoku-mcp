# progress-turn Delta Specification (seasonal-events)

## MODIFIED Requirements

### Requirement: ターンフェーズと季節イベント
システムは、ターンベースのループを処理し、すべての領地に対して季節イベントを適用しなければならない (MUST)。

#### Scenario: 全大名の行動完了と季節イベント
- **WHEN** ターン内のすべての大名が行動を完了した時
- **THEN** ターンカウンターが増加し、**新機能「seasonal-events」に基づく季節イベント処理が実行され**、次のターンの処理が開始される

#### Scenario: 資源生成の季節 (DEPRECATED)
- **THEN** (このロジックは seasonal-events 側に統合・移行される)

#### Scenario: ランダムな災害 (DEPRECATED)
- **THEN** (このロジックは seasonal-events 側に統合・移行される)
