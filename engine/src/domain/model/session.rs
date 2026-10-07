use crate::domain::model::action_log::ActionLogEntry;
use crate::domain::model::battle::WarStatus;
use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::game_state::GameState;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{DaimyoId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 永続化対象のゲームセッションデータ（スナップショット）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// セッションの一意識別子
    pub session_id: SessionId,
    /// 作成日時 (UTC)
    pub created_at: DateTime<Utc>,
    /// 最終アクセス日時 (UTC)
    pub last_accessed_at: DateTime<Utc>,
    /// 選択中の大名ID
    pub selected_daimyo_id: Option<DaimyoId>,
    /// ゲーム進行状態
    pub game_state: Option<GameState>,
    /// 全領地データ
    pub kunis: Vec<Kuni>,
    /// 全大名データ
    pub daimyos: Vec<Daimyo>,
    /// 進行中の合戦データ
    pub battles: Vec<WarStatus>,
    /// 行動ログ
    pub action_logs: Vec<ActionLogEntry>,
}

impl SessionData {
    /// 新規セッションデータを初期化します
    pub fn new(
        session_id: impl Into<SessionId>,
        selected_daimyo_id: Option<DaimyoId>,
        game_state: Option<GameState>,
        kunis: Vec<Kuni>,
        daimyos: Vec<Daimyo>,
        battles: Vec<WarStatus>,
        action_logs: Vec<ActionLogEntry>,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            created_at: now,
            last_accessed_at: now,
            selected_daimyo_id,
            game_state,
            kunis,
            daimyos,
            battles,
            action_logs,
        }
    }

    /// アクセス日時を現在時刻に更新します
    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }
}
