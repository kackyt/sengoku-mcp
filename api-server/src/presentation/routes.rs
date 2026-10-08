use crate::application::dto::{CreatedGameDto, MyStatusDto};
use crate::application::{GameCreationService, StatusQueryError, StatusQueryService};
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use engine::domain::model::value_objects::SessionId;
use serde::Serialize;
use std::sync::Arc;

/// MCPサーバーで session_id を省略した場合に使われるセッションID
const DEFAULT_SESSION_ID: &str = "default";

/// エラーレスポンスの本体
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// 機械判定用のエラーコード
    code: &'static str,
    /// 人間向けのメッセージ
    message: String,
}

/// ルーターが保持するアプリケーションサービス群
#[derive(Clone)]
pub struct AppState {
    pub status: Arc<StatusQueryService>,
    pub games: Arc<GameCreationService>,
}

impl FromRef<AppState> for Arc<StatusQueryService> {
    fn from_ref(state: &AppState) -> Self {
        state.status.clone()
    }
}

impl FromRef<AppState> for Arc<GameCreationService> {
    fn from_ref(state: &AppState) -> Self {
        state.games.clone()
    }
}

/// アプリケーション層のエラーを HTTP レスポンスへ変換するラッパー
struct ApiError(StatusQueryError);

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self(StatusQueryError::Internal(e))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            StatusQueryError::ViewTokenNotFound => (StatusCode::NOT_FOUND, "view_not_found"),
            StatusQueryError::SessionNotFound(_) => (StatusCode::NOT_FOUND, "session_not_found"),
            StatusQueryError::DaimyoNotSelected(_) => (StatusCode::CONFLICT, "daimyo_not_selected"),
            StatusQueryError::WaitingForJoin => (StatusCode::CONFLICT, "waiting_for_join"),
            StatusQueryError::Internal(e) => {
                // 内部エラーの詳細はログにのみ出力する
                eprintln!("[Sengoku-API] Internal error: {:#}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };
        let message = match &self.0 {
            StatusQueryError::Internal(_) => "内部エラーが発生しました".to_string(),
            other => other.to_string(),
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

/// ルーターを構築します
///
/// - `GET /health` : ヘルスチェック
/// - `POST /api/games` : 新規ゲームを作成し、閲覧トークンと参加コードを発行
/// - `GET /api/views/{token}/status` : 閲覧トークンに対応する自国の状況
/// - `GET /api/status` : デフォルトセッションの自国の状況
/// - `GET /api/sessions/{session_id}/status` : 指定セッションの自国の状況
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/games", post(create_game))
        .route("/api/views/{token}/status", get(view_status))
        .route("/api/status", get(default_status))
        .route("/api/sessions/{session_id}/status", get(session_status))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn create_game(
    State(service): State<Arc<GameCreationService>>,
) -> Result<(StatusCode, Json<CreatedGameDto>), ApiError> {
    let created = service.create_game().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn view_status(
    State(service): State<Arc<StatusQueryService>>,
    Path(token): Path<String>,
) -> Result<Json<MyStatusDto>, ApiError> {
    service
        .get_my_status_by_token(&token)
        .await
        .map(Json)
        .map_err(ApiError)
}

async fn default_status(
    State(service): State<Arc<StatusQueryService>>,
) -> Result<Json<MyStatusDto>, ApiError> {
    my_status(&service, SessionId::new(DEFAULT_SESSION_ID)).await
}

async fn session_status(
    State(service): State<Arc<StatusQueryService>>,
    Path(session_id): Path<String>,
) -> Result<Json<MyStatusDto>, ApiError> {
    // MCPサーバーと同様に前後の空白を除去し、空ならデフォルトセッションとして扱う
    let trimmed = session_id.trim();
    let session_id = if trimmed.is_empty() {
        SessionId::new(DEFAULT_SESSION_ID)
    } else {
        SessionId::new(trimmed)
    };
    my_status(&service, session_id).await
}

/// 自国の状況を取得して JSON で返す共通処理
async fn my_status(
    service: &StatusQueryService,
    session_id: SessionId,
) -> Result<Json<MyStatusDto>, ApiError> {
    service
        .get_my_status(&session_id)
        .await
        .map(Json)
        .map_err(ApiError)
}
