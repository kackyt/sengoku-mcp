//! ゲームセッション（GameContext）の構築とライフサイクル管理を行うクレート
//!
//! MCPサーバーとREST APIサーバーの双方から利用され、
//! `SessionRepository`（ファイル / GCS）を介してゲーム状態を共有します。

pub mod game_context;
pub mod session_manager;

pub use game_context::{GameContext, GameContextFactory};
pub use session_manager::SessionManager;
