use crate::domain::error::DomainError;
use crate::domain::model::event::GameEvent;

/// ドメインイベントを外部システムに通知するディスパッチャのインターフェース
#[async_trait::async_trait]
pub trait EventDispatcher: Send + Sync {
    /// イベントを送信します
    async fn dispatch(&self, event: GameEvent) -> Result<(), DomainError>;
    /// 複数のイベントをまとめて送信します
    async fn dispatch_all(&self, events: Vec<GameEvent>) -> Result<(), DomainError> {
        for event in events {
            self.dispatch(event).await?;
        }
        Ok(())
    }
}
