use crate::domain::model::event::GameEvent;

/// ドメインイベントを外部システムに通知するディスパッチャのインターフェース
#[async_trait::async_trait]
pub trait EventDispatcher: Send + Sync {
    /// イベントを送信します
    async fn dispatch(&self, event: GameEvent);
    /// 複数のイベントをまとめて送信します
    async fn dispatch_all(&self, events: Vec<GameEvent>) {
        for event in events {
            self.dispatch(event).await;
        }
    }
}
