# manage-domestic Delta Specification

## MODIFIED Requirements

### Requirement: 委任およびCPU行動の高度化
CPUおよび委任された領地の行動決定ロジックを、完全ランダムから戦略的なスコアリング方式に変更する。

#### Scenario: 委任領地の自動開発
- **WHEN** 領地が委任（Inin）状態にある時
- **THEN** 新しい `cpu-ai-logic` に基づき、周囲の脅威度に応じた雇用や開発が自動で行われる
