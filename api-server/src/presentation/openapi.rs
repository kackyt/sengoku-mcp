use crate::application::dto::{
    CreatedGameDto, DaimyoDto, KuniStatusDto, MyStatusDto, OtherKuniDto,
};
use crate::presentation::routes::{self, ErrorCode, ErrorResponse};
use utoipa::OpenApi;

/// REST API の OpenAPI 定義
///
/// 各エンドポイント・スキーマの説明は、ハンドラーと DTO のドキュメントコメントから生成されます。
/// リポジトリには生成結果を `api-server/openapi.json` として保存しており、
/// `UPDATE_OPENAPI=1 cargo test -p api-server --test openapi_test` で更新できます。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sengoku REST API",
        license(name = "MIT", identifier = "MIT"),
        description = "MCPサーバー（sengoku-mcp）と保存先を共有し、自国の状況を返す REST API です。\n\n\
Webアプリは `POST /api/games` でゲームを作成し、発行された参加コードをプレイヤーがチャットで伝えることで、\
LLM（MCP）側のゲームと連携します。状況の取得には閲覧トークンを使用します。"
    ),
    // 待ち受け先は環境により異なるため、ローカル起動時のデフォルトを例示する
    servers((url = "http://localhost:8080", description = "ローカル起動時のデフォルト")),
    // 現状は認証なし（公開時は Cloud Run の IAM 認証等で保護する）
    security(()),
    paths(
        routes::health,
        routes::create_game,
        routes::view_status,
        routes::default_status,
        routes::session_status,
    ),
    components(schemas(
        MyStatusDto,
        DaimyoDto,
        KuniStatusDto,
        OtherKuniDto,
        CreatedGameDto,
        ErrorResponse,
        ErrorCode,
    )),
    tags(
        (name = "games", description = "Webでのゲーム作成"),
        (name = "status", description = "自国の状況の取得"),
        (name = "system", description = "運用"),
    )
)]
pub struct ApiDoc;
