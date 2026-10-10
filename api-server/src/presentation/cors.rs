use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// 許可するオリジンを指定する環境変数名（カンマ区切り。`*` で全オリジンを許可）
pub const ENV_CORS_ALLOW_ORIGINS: &str = "SENGOKU_CORS_ALLOW_ORIGINS";

/// 許可オリジンの設定値から CORS レイヤーを構築します
///
/// Webアプリを API と別オリジン（例: 静的ホスティング）で配信する場合に使用します。
/// 未指定・空の場合は `None`（CORS ヘッダーを付けない＝同一オリジンのみ）を返します。
/// 不正なオリジン文字列は無視します。
pub fn cors_layer(allow_origins: Option<&str>) -> Option<CorsLayer> {
    let value = allow_origins.map(str::trim).filter(|v| !v.is_empty())?;
    let base = CorsLayer::new().allow_methods([Method::GET, Method::POST]);

    if value == "*" {
        return Some(base.allow_origin(tower_http::cors::Any));
    }
    let origins: Vec<HeaderValue> = value
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    (!origins.is_empty()).then(|| base.allow_origin(origins))
}
