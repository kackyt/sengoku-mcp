use crate::domain::error::DomainError;
use crate::domain::model::value_objects::{ActionOrderIndex, DaimyoId, TurnNumber};

/// ゲームの進行状態全体を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    /// 現在のターン（季節）
    current_turn: TurnNumber,
    /// このターンで行動する大名の順番
    action_order: Vec<DaimyoId>,
    /// 現在行動中の大名のインデックス（`action_order`のインデックス）
    current_action_index: ActionOrderIndex,
}

impl GameState {
    /// 新規ゲーム状態を生成します。
    pub fn new(
        current_turn: TurnNumber,
        action_order: Vec<DaimyoId>,
        current_action_index: ActionOrderIndex,
    ) -> Result<Self, DomainError> {
        if current_action_index.value() > action_order.len() {
            return Err(DomainError::ValidationError(
                "current_action_index must be <= action_order.len()".to_string(),
            ));
        }
        Ok(Self {
            current_turn,
            action_order,
            current_action_index,
        })
    }

    pub fn current_turn(&self) -> TurnNumber {
        self.current_turn
    }

    pub fn action_order(&self) -> &[DaimyoId] {
        &self.action_order
    }

    pub fn current_action_index(&self) -> ActionOrderIndex {
        self.current_action_index
    }

    /// 現在行動中の大名IDを取得します。
    /// 順番が終了している場合は `None` を返します。
    pub fn current_daimyo(&self) -> Option<DaimyoId> {
        self.action_order
            .get(self.current_action_index.value())
            .copied()
    }

    /// 次の行動大名に進みます。
    /// すでに最後の大名だった場合、インデックスは要素数と同じになり、`current_daimyo` は `None` を返します。
    pub fn advance_action(&mut self) {
        if self.current_action_index.value() < self.action_order.len() {
            self.current_action_index =
                ActionOrderIndex::new(self.current_action_index.value() + 1);
        }
    }

    /// ターン内のすべての大名が行動を完了したかを判定します。
    pub fn is_turn_completed(&self) -> bool {
        self.current_action_index.value() >= self.action_order.len()
    }

    /// 新しいターンを開始し、行動順序をリセットします。
    pub fn start_new_turn(&mut self, new_order: Vec<DaimyoId>) {
        self.current_turn = TurnNumber::new(self.current_turn.value() + 1);
        self.action_order = new_order;
        self.current_action_index = ActionOrderIndex::new(0);
    }
}
