# progress-turn Specification

## Purpose
TBD - created by archiving change implement-domain-logic. Update Purpose after archive.

## Requirements
### Requirement: ターンフェーズと季節イベント
システムは、ターンベースのループを処理し、すべての領地に対して季節イベントを適用しなければならない (MUST)。

#### Scenario: ターンの開始と行動順序の決定
- **WHEN** 新しいターンが開始された時
- **THEN** 各大名の行動順序がランダムに決定され、最初の大名の行動フェーズに移行する

#### Scenario: 行動の完了と次の大名への移行
- **WHEN** 現在の大名が行動を完了した時
- **THEN** 次の順序の大名の行動フェーズに移行する

#### Scenario: 全大名の行動完了と季節イベント
- **WHEN** ターン内のすべての大名が行動を完了した時
- **THEN** ターンカウンターが増加し、新機能「seasonal-events」に基づく季節イベント処理（資源生成、災害、反乱など）が実行され、次のターンの処理が開始される

#### Scenario: 資源生成の季節 (DEPRECATED)
- **THEN** (このロジックは seasonal-events 側に統合・移行されました)

#### Scenario: ランダムな災害 (DEPRECATED)
- **THEN** (このロジックは seasonal-events 側に統合・移行されました)

### Requirement: 自動行動（CPU）とイベント発行
システムは、各行動番の大名について自動的に行動（CPU行動）を決定・実行し、その結果をイベントとして発行しなければならない (MUST)。

#### Scenario: CPUによる自動内政・戦争
- **WHEN** 大名の行動番になった時
- **THEN** システムは資金・兵力・資源の状況に応じて自動的にコマンド（戦争を含む）を実行し、その行動と結果をイベントとして発行する

#### Scenario: 状態進行のイベント通知
- **WHEN** 行動大名が移行した時、またはターン（季節）が進行した時
- **THEN** システムは現在のターン数と行動順に関する状態更新イベントを発行する
