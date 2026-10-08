use crate::application::dto::{CreatedGameDto, MyStatusDto};
use crate::application::{GameCreationService, StatusQueryError, StatusQueryService};
use crate::presentation::openapi::ApiDoc;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use engine::domain::model::value_objects::SessionId;
use serde::Serialize;
use std::sync::Arc;
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

/// MCPサーバーで session_id を省略した場合に使われるセッションID
const DEFAULT_SESSION_ID: &str = "default";

/// 機械判定用のエラーコード
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 閲覧トークンが不正・未発行・失効済み、またはゲームが期限切れで削除済み（404）
    ViewNotFound,
    /// 指定したセッションが存在しない（404）
    SessionNotFound,
    /// 大名がまだ選択されていない（409）
    DaimyoNotSelected,
    /// Webで作成したゲームに、チャット側がまだ参加していない（409）
    WaitingForJoin,
    /// 保存先へのアクセス失敗など、サーバー内部のエラー（500）
    InternalError,
}

/// エラーレスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// 機械判定用のエラーコード
    pub code: ErrorCode,
    /// 人間向けのメッセージ（日本語）
    #[schema(example = "チャット側の参加待ちです。チャットで参加コードを伝えてください")]
    pub message: String,
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
pub(crate) struct ApiError(StatusQueryError);

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self(StatusQueryError::Internal(e))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            StatusQueryError::ViewTokenNotFound => (StatusCode::NOT_FOUND, ErrorCode::ViewNotFound),
            StatusQueryError::SessionNotFound(_) => {
                (StatusCode::NOT_FOUND, ErrorCode::SessionNotFound)
            }
            StatusQueryError::DaimyoNotSelected(_) => {
                (StatusCode::CONFLICT, ErrorCode::DaimyoNotSelected)
            }
            StatusQueryError::WaitingForJoin => (StatusCode::CONFLICT, ErrorCode::WaitingForJoin),
            StatusQueryError::Internal(e) => {
                // 内部エラーの詳細はログにのみ出力する
                eprintln!("[Sengoku-API] Internal error: {:#}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
            }
        };
        let message = match &self.0 {
            StatusQueryError::Internal(_) => "内部エラーが発生しました".to_string(),
            other => other.to_string(),
        };
        (status, Json(ErrorResponse { code, message })).into_response()
    }
}

/// ルーターを構築します
///
/// API本体に加え、OpenAPI仕様（`GET /openapi.json`）と API ドキュメント画面（`GET /docs`）を公開します。
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/games", post(create_game))
        .route("/api/views/{token}/status", get(view_status))
        .route("/api/status", get(default_status))
        .route("/api/sessions/{session_id}/status", get(session_status))
        .route("/openapi.json", get(openapi_json))
        .with_state(state)
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
}

/// OpenAPI仕様を JSON で返します
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// ヘルスチェック
///
/// サーバーが起動していれば `ok` を返します。保存先への疎通は確認しません。
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses((status = 200, description = "稼働中", body = String, content_type = "text/plain", example = "ok"))
)]
pub(crate) async fn health() -> &'static str {
    "ok"
}

/// 新規ゲームを作成する
///
/// 参加待ちのゲームを作成し、ブラウザ用の閲覧トークン（`status_url`）と、
/// チャット（LLM）に伝える参加コード（6文字・30分有効・1回限り）を発行します。
///
/// 1. ブラウザは `status_url` を保持してポーリングします（参加前は 409 `waiting_for_join`）。
/// 2. プレイヤーがチャットで参加コードを伝えると、LLM が MCP ツール `join_game` を呼び出します。
/// 3. 大名が選択されると、`status_url` で自国の状況が取得できるようになります（200）。
#[utoipa::path(
    post,
    path = "/api/games",
    tag = "games",
    responses(
        (status = 201, description = "ゲームを作成しました", body = CreatedGameDto),
        (status = 500, description = "保存先へのアクセスに失敗しました（`internal_error`）", body = ErrorResponse),
    )
)]
pub(crate) async fn create_game(
    State(service): State<Arc<GameCreationService>>,
) -> Result<(StatusCode, Json<CreatedGameDto>), ApiError> {
    let created = service.create_game().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// 閲覧トークンで自国の状況を取得する
///
/// Webアプリ向けのエンドポイントです。`POST /api/games` で発行された閲覧トークン、
/// またはMCPツールの結果に付与された閲覧URLのトークンで、対応するゲームの自国の状況を返します。
/// レスポンスにはセッションID（チャットID等）は含まれません。
#[utoipa::path(
    get,
    path = "/api/views/{token}/status",
    tag = "status",
    params(("token" = String, Path, description = "閲覧トークン（32桁の16進数）", example = "777fc6d170d94495b49babeba2230e7d")),
    responses(
        (status = 200, description = "自国の状況", body = MyStatusDto),
        (status = 404, description = "閲覧トークンが無効です（`view_not_found`）", body = ErrorResponse),
        (status = 409, description = "チャット側の参加待ち（`waiting_for_join`）、または大名が未選択（`daimyo_not_selected`）", body = ErrorResponse),
        (status = 500, description = "サーバー内部のエラー（`internal_error`）", body = ErrorResponse),
    )
)]
pub(crate) async fn view_status(
    State(service): State<Arc<StatusQueryService>>,
    Path(token): Path<String>,
) -> Result<Json<MyStatusDto>, ApiError> {
    service
        .get_my_status_by_token(&token)
        .await
        .map(Json)
        .map_err(ApiError)
}

/// defaultセッションの自国の状況を取得する
///
/// MCPツールで `session_id` を省略して遊んでいる場合（`default` セッション）の自国の状況を返します。
/// セッションIDだけで参照できるため、公開環境ではアクセス制限を推奨します。
#[utoipa::path(
    get,
    path = "/api/status",
    tag = "status",
    responses(
        (status = 200, description = "自国の状況", body = MyStatusDto),
        (status = 404, description = "セッションが存在しません（`session_not_found`）", body = ErrorResponse),
        (status = 409, description = "大名が未選択です（`daimyo_not_selected`）", body = ErrorResponse),
        (status = 500, description = "サーバー内部のエラー（`internal_error`）", body = ErrorResponse),
    )
)]
pub(crate) async fn default_status(
    State(service): State<Arc<StatusQueryService>>,
) -> Result<Json<MyStatusDto>, ApiError> {
    my_status(&service, SessionId::new(DEFAULT_SESSION_ID)).await
}

/// セッションIDで自国の状況を取得する
///
/// セッションID（MCPツールの `session_id` と同じ値）を知っているクライアント・デバッグ向けです。
/// Webアプリからは閲覧トークン（`/api/views/{token}/status`）を使用してください。
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/status",
    tag = "status",
    params(("session_id" = String, Path, description = "セッションID（前後の空白は除去。空なら `default`）", example = "discord_12345")),
    responses(
        (status = 200, description = "自国の状況", body = MyStatusDto),
        (status = 404, description = "セッションが存在しません（`session_not_found`）", body = ErrorResponse),
        (status = 409, description = "大名が未選択です（`daimyo_not_selected`）", body = ErrorResponse),
        (status = 500, description = "サーバー内部のエラー（`internal_error`）", body = ErrorResponse),
    )
)]
pub(crate) async fn session_status(
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
