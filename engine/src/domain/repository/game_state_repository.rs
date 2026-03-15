use crate::domain::error::DomainError;
use crate::domain::model::game_state::GameState;

/// ゲーム状態を管理するリポジトリのインターフェース
#[async_trait::async_trait]
pub trait GameStateRepository: Send + Sync {
    /// 現在のゲーム状態を取得します
    async fn get(&self) -> Result<Option<GameState>, DomainError>;
    /// ゲーム状態を保存または更新します
    async fn save(&self, state: &GameState) -> Result<(), DomainError>;
}
