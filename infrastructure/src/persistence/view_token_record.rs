use chrono::{DateTime, Utc};
use engine::domain::model::value_objects::{SessionId, ViewToken};
use serde::{Deserialize, Serialize};

/// 閲覧トークンの対応表を保存するサブディレクトリ（プレフィックス）名
///
/// セッションJSONと同じ保存先の配下に置きますが、セッションの期限切れ
/// クリーンアップは直下のファイルのみを対象とするため、ここは対象外になります。
pub(crate) const VIEW_TOKEN_DIR: &str = "view_tokens";

/// 閲覧トークンの保存先ファイル名を返します（トークンは形式検証済みのため無害化不要）
pub(crate) fn view_token_file_name(token: &ViewToken) -> String {
    format!("{}.json", token.value())
}

/// 閲覧トークン1件分の保存形式
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewTokenRecord {
    /// 対応するセッションID
    pub session_id: SessionId,
    /// 発行日時
    pub created_at: DateTime<Utc>,
}

impl ViewTokenRecord {
    pub fn new(session_id: &SessionId) -> Self {
        Self {
            session_id: session_id.clone(),
            created_at: Utc::now(),
        }
    }
}
