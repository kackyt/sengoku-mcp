use crate::domain::model::value_objects::DaimyoId;

/// ゲームの進行状態全体を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    /// 現在のターン（季節）
    pub current_turn: u32,
    /// このターンで行動する大名の順番
    pub action_order: Vec<DaimyoId>,
    /// 現在行動中の大名のインデックス（`action_order`のインデックス）
    pub current_action_index: usize,
}

impl GameState {
    /// 新規ゲーム状態を生成します。
    pub fn new(current_turn: u32, action_order: Vec<DaimyoId>, current_action_index: usize) -> Self {
        Self {
            current_turn,
            action_order,
            current_action_index,
        }
    }

    /// 現在行動中の大名IDを取得します。
    /// 順番が終了している場合は `None` を返します。
    pub fn current_daimyo(&self) -> Option<DaimyoId> {
        self.action_order.get(self.current_action_index).copied()
    }

    /// 次の行動大名に進みます。
    /// すでに最後の大名だった場合、インデックスは要素数と同じになり、`current_daimyo` は `None` を返します。
    pub fn advance_action(&mut self) {
        if self.current_action_index < self.action_order.len() {
            self.current_action_index += 1;
        }
    }

    /// ターン内のすべての大名が行動を完了したかを判定します。
    pub fn is_turn_completed(&self) -> bool {
        self.current_action_index >= self.action_order.len()
    }

    /// 新しいターンを開始し、行動順序をリセットします。
    pub fn start_new_turn(&mut self, new_order: Vec<DaimyoId>) {
        self.current_turn += 1;
        self.action_order = new_order;
        self.current_action_index = 0;
    }
}
