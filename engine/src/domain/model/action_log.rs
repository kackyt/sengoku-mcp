use crate::domain::model::value_objects::TurnNumber;

/// ログのカテゴリを表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionLogCategory {
    /// 内政フェーズのイベント
    Domestic,
    /// 合戦フェーズのイベント
    War,
}

/// ログの公開範囲（Visibility）を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLogVisibility {
    /// 全ての情報をCLIに表示する（季節イベント、合戦決着等）
    Public,
    /// 操作プレイヤーに係るイベントのみCLIに表示する
    Player,
    /// 詳細に記録するがCLIには表示しない（CPU行動、詳細計算値等チート防止用）
    Internal,
}

/// アクションログのエントリを表現するモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionLogEntry {
    /// ログのカテゴリ（内政・合戦）
    pub category: ActionLogCategory,
    /// 公開範囲
    pub visibility: ActionLogVisibility,
    /// 発生ターン
    pub turn: TurnNumber,
    /// CLIに表示する短いメッセージ（Public/Playerの場合のみ表示対象）
    pub message: String,
    /// 詳細ログ（デバッグ・記録用、常に記録）
    pub detail: String,
}

impl ActionLogEntry {
    pub fn new(
        category: ActionLogCategory,
        visibility: ActionLogVisibility,
        turn: TurnNumber,
        message: String,
        detail: String,
    ) -> Self {
        Self {
            category,
            visibility,
            turn,
            message,
            detail,
        }
    }
}
