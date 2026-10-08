// セッション管理はREST APIサーバーと共有するため game-session クレートへ切り出している
pub use game_session::{GameContext, SessionManager};
