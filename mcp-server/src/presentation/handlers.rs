use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use std::sync::Arc;

#[allow(dead_code)]
pub struct McpHandlers {
    turn_progression_usecase: Arc<TurnProgressionUseCase>,
}

#[allow(dead_code)]
impl McpHandlers {
    pub fn new(turn_progression_usecase: Arc<TurnProgressionUseCase>) -> Self {
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
