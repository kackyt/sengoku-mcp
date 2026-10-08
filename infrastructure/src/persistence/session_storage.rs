use crate::persistence::object_store_session_repository::{
    ObjectStoreSessionError, ObjectStoreSessionRepository,
};
use crate::persistence::session_persistence::SessionPersistenceManager;
use engine::domain::repository::session_repository::SessionRepository;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

/// 保存先の種別を指定する環境変数名（`file` または `gcs`、省略時は `file`）
pub const ENV_STORAGE: &str = "SENGOKU_STORAGE";
/// GCS バケット名を指定する環境変数名
pub const ENV_GCS_BUCKET: &str = "SENGOKU_GCS_BUCKET";
/// GCS 上のオブジェクトキーのプレフィックスを指定する環境変数名
pub const ENV_GCS_PREFIX: &str = "SENGOKU_GCS_PREFIX";
/// GCS のプレフィックスのデフォルト値
pub const DEFAULT_GCS_PREFIX: &str = "sessions";

/// セッションストレージ設定の解決・構築に関するエラー
#[derive(Debug, Error)]
pub enum SessionStorageError {
    #[error("{ENV_STORAGE} の値が不正です: {0}（file または gcs を指定してください）")]
    UnknownStorage(String),
    #[error("{ENV_STORAGE}=gcs の場合は {ENV_GCS_BUCKET} を指定してください")]
    MissingBucket,
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreSessionError),
}

/// セッションデータの保存先設定
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStorageConfig {
    /// ローカルファイルシステム（ディレクトリ配下に JSON ファイルとして保存）
    File { dir: PathBuf },
    /// Google Cloud Storage（`gs://<bucket>/<prefix>/<セッションID>.json` として保存）
    Gcs { bucket: String, prefix: String },
}

impl SessionStorageConfig {
    /// 環境変数から設定を解決します
    ///
    /// - `SENGOKU_STORAGE`: `file`（デフォルト）または `gcs`
    /// - `SENGOKU_SESSIONS_DIR`: `file` の保存先ディレクトリ（デフォルト: `data/sessions`）
    /// - `SENGOKU_GCS_BUCKET`: `gcs` のバケット名（必須）
    /// - `SENGOKU_GCS_PREFIX`: `gcs` のキーのプレフィックス（デフォルト: `sessions`）
    pub fn from_env() -> Result<Self, SessionStorageError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// 任意の値取得関数から設定を解決します（テスト容易性のため環境変数アクセスを分離）
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, SessionStorageError> {
        // 空文字は未指定として扱う
        let get = |key: &str| {
            lookup(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        match get(ENV_STORAGE).map(|v| v.to_ascii_lowercase()).as_deref() {
            None | Some("file") => Ok(Self::File {
                dir: SessionPersistenceManager::default_dir(),
            }),
            Some("gcs") => Ok(Self::Gcs {
                bucket: get(ENV_GCS_BUCKET).ok_or(SessionStorageError::MissingBucket)?,
                prefix: get(ENV_GCS_PREFIX)
                    .map(|p| p.trim_matches('/').to_string())
                    .unwrap_or_else(|| DEFAULT_GCS_PREFIX.to_string()),
            }),
            Some(other) => Err(SessionStorageError::UnknownStorage(other.to_string())),
        }
    }

    /// 設定に対応するセッションリポジトリを構築します
    pub fn build(&self) -> Result<Arc<dyn SessionRepository>, SessionStorageError> {
        match self {
            Self::File { dir } => Ok(Arc::new(SessionPersistenceManager::new(dir.clone()))),
            Self::Gcs { bucket, prefix } => {
                Ok(Arc::new(ObjectStoreSessionRepository::gcs(bucket, prefix)?))
            }
        }
    }

    /// ログ出力用の保存先の説明を返します
    pub fn describe(&self) -> String {
        match self {
            Self::File { dir } => format!("file://{}", dir.display()),
            Self::Gcs { bucket, prefix } => format!("gs://{}/{}", bucket, prefix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// HashMap を値取得関数として使うヘルパー
    fn resolve(vars: &[(&str, &str)]) -> Result<SessionStorageConfig, SessionStorageError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        SessionStorageConfig::from_lookup(|key| map.get(key).cloned())
    }

    #[test]
    fn test_default_is_file() {
        assert!(matches!(
            resolve(&[]).unwrap(),
            SessionStorageConfig::File { .. }
        ));
        assert!(matches!(
            resolve(&[(ENV_STORAGE, "FILE")]).unwrap(),
            SessionStorageConfig::File { .. }
        ));
    }

    #[test]
    fn test_gcs_config() {
        let config = resolve(&[
            (ENV_STORAGE, "gcs"),
            (ENV_GCS_BUCKET, "my-bucket"),
            (ENV_GCS_PREFIX, "/prod/sessions/"),
        ])
        .unwrap();
        assert_eq!(
            config,
            SessionStorageConfig::Gcs {
                bucket: "my-bucket".to_string(),
                prefix: "prod/sessions".to_string(),
            }
        );
        assert_eq!(config.describe(), "gs://my-bucket/prod/sessions");

        // プレフィックス省略時はデフォルト値
        let config = resolve(&[(ENV_STORAGE, "gcs"), (ENV_GCS_BUCKET, "b")]).unwrap();
        assert_eq!(
            config,
            SessionStorageConfig::Gcs {
                bucket: "b".to_string(),
                prefix: DEFAULT_GCS_PREFIX.to_string(),
            }
        );
    }

    #[test]
    fn test_gcs_requires_bucket() {
        assert!(matches!(
            resolve(&[(ENV_STORAGE, "gcs"), (ENV_GCS_BUCKET, " ")]),
            Err(SessionStorageError::MissingBucket)
        ));
    }

    #[test]
    fn test_unknown_storage() {
        assert!(matches!(
            resolve(&[(ENV_STORAGE, "s3")]),
            Err(SessionStorageError::UnknownStorage(_))
        ));
    }

    #[test]
    fn test_build_file_repository() {
        let dir = tempfile::tempdir().unwrap();
        let config = SessionStorageConfig::File {
            dir: dir.path().to_path_buf(),
        };
        assert!(config.build().is_ok());
    }
}
