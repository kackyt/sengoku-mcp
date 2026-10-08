//! 自国の状況を返す REST API サーバー
//!
//! MCPサーバーと同じ `SessionRepository`（ファイル / GCS）を参照することで、
//! MCP経由で進行したゲーム状態を読み取り専用で公開します。

pub mod application;
pub mod presentation;
