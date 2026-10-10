use crate::application::dto::CreatedGameDto;
use game_session::GameLobby;

/// Webからの新規ゲーム作成を扱うアプリケーションサービス
pub struct GameCreationService {
    lobby: GameLobby,
}

impl GameCreationService {
    pub fn new(lobby: GameLobby) -> Self {
        Self { lobby }
    }

    /// 参加待ちのゲームを作成し、ブラウザ向けの閲覧トークンとチャット向けの参加コードを返します
    pub async fn create_game(&self) -> anyhow::Result<CreatedGameDto> {
        let created = self.lobby.create_game().await?;
        let join_code = created.join_code.value().to_string();
        Ok(CreatedGameDto {
            status_url: format!("/api/views/{}/status", created.view_token.value()),
            view_token: created.view_token.value().to_string(),
            join_message: format!(
                "チャットで「参加コード {} でゲームに参加して」と伝えてください",
                join_code
            ),
            join_code,
            join_code_expires_at: created.join_code_expires_at,
        })
    }
}
