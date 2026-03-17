use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use engine::domain::repository::{
    daimyo_repository::DaimyoRepository, event_dispatcher::EventDispatcher,
    game_state_repository::GameStateRepository, kuni_repository::KuniRepository,
};
use std::sync::Arc;

#[allow(dead_code)]
pub struct McpHandlers<
    KR: KuniRepository + 'static,
    DR: DaimyoRepository + 'static,
    GSR: GameStateRepository + 'static,
    ED: EventDispatcher + 'static,
> {
    turn_progression_usecase: Arc<TurnProgressionUseCase<KR, DR, GSR, ED>>,
}

#[allow(dead_code)]
impl<KR, DR, GSR, ED> McpHandlers<KR, DR, GSR, ED>
where
    KR: KuniRepository,
    DR: DaimyoRepository,
    GSR: GameStateRepository,
    ED: EventDispatcher,
{
    pub fn new(turn_progression_usecase: Arc<TurnProgressionUseCase<KR, DR, GSR, ED>>) -> Self {
        Self {
            turn_progression_usecase,
        }
    }

    /// 「ターンを進める（自動行動を1ステップ進める）」ツールのハンドラ
    pub async fn handle_progress_turn(&self) -> Result<String, anyhow::Error> {
        self.turn_progression_usecase.progress().await?;
        // 今回の仕様ではイベントとして状態を送信するため、戻り値は簡易な完了メッセージとする
        Ok(
            "ゲームの進行処理（１ステップ）を実行しました。イベントを確認してください。"
                .to_string(),
        )
    }
}
