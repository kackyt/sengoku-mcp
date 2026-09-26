use chrono::{DateTime, Duration, Utc};
use engine::domain::error::DomainError;
pub use engine::domain::model::session::SessionData;
use engine::domain::model::value_objects::SessionId;
use engine::domain::repository::session_repository::SessionRepository;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// セッション永続化処理に関する型安全なカスタムエラー
#[derive(Debug, Error)]
pub enum SessionPersistenceError {
    #[error("セッションファイルのI/Oエラー: {0}")]
    IoError(#[from] std::io::Error),
    #[error("セッションデータのJSON変換エラー: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// セッションファイルの永続化・クリーンアップを管理するマネージャー
#[derive(Debug, Clone)]
pub struct SessionPersistenceManager {
    storage_dir: PathBuf,
}

impl SessionPersistenceManager {
    /// 保存先ディレクトリを指定してインスタンスを生成します
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            storage_dir: storage_dir.into(),
        }
    }

    /// デフォルトの保存先ディレクトリ（data/sessions）で生成します
    pub fn default_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("SENGOKU_SESSIONS_DIR") {
            PathBuf::from(dir)
        } else {
            PathBuf::from("data/sessions")
        }
    }

    /// ディレクトリが存在することを確認し、なければ作成します
    fn ensure_dir(&self) -> std::io::Result<()> {
        if !self.storage_dir.exists() {
            fs::create_dir_all(&self.storage_dir)?;
        }
        Ok(())
    }

    /// セッションIDから安全なファイルパスを取得します（ディレクトリトラバーサル防止）
    fn session_file_path(&self, session_id: &SessionId) -> PathBuf {
        let safe_name: String = session_id
            .value()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.storage_dir.join(format!("{}.json", safe_name))
    }

    /// セッションデータをファイルにアトミック保存します
    pub fn save(&self, data: &SessionData) -> Result<(), SessionPersistenceError> {
        self.ensure_dir()?;
        let target_path = self.session_file_path(&data.session_id);
        let tmp_path = target_path.with_extension("tmp");

        let json = serde_json::to_string_pretty(data)?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &target_path)?;

        Ok(())
    }

    /// セッションデータをファイルから読み込みます
    pub fn load(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionData>, SessionPersistenceError> {
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let data: SessionData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// セッションファイルを削除します
    pub fn delete(&self, session_id: &SessionId) -> Result<bool, SessionPersistenceError> {
        let path = self.session_file_path(session_id);
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 期限切れ（最終アクセスからttl以上経過）したセッションファイルを削除します
    pub fn cleanup_expired(&self, ttl: Duration) -> Result<usize, SessionPersistenceError> {
        if !self.storage_dir.exists() {
            return Ok(0);
        }

        let now = Utc::now();
        let mut deleted_count = 0;

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                let should_delete = match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<SessionData>(&content) {
                        Ok(data) => (now - data.last_accessed_at) > ttl,
                        Err(_) => Self::is_file_expired(&path, ttl, now),
                    },
                    Err(_) => Self::is_file_expired(&path, ttl, now),
                };

                if should_delete {
                    if let Ok(()) = fs::remove_file(&path) {
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    fn is_file_expired(path: &Path, ttl: Duration, now: DateTime<Utc>) -> bool {
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let modified_utc: DateTime<Utc> = modified.into();
                return (now - modified_utc) > ttl;
            }
        }
        false
    }
}

impl Default for SessionPersistenceManager {
    fn default() -> Self {
        Self::new(Self::default_dir())
    }
}

#[async_trait::async_trait]
impl SessionRepository for SessionPersistenceManager {
    async fn save(&self, data: &SessionData) -> Result<(), DomainError> {
        self.save(data)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionData>, DomainError> {
        self.load(session_id)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn delete(&self, session_id: &SessionId) -> Result<bool, DomainError> {
        self.delete(session_id)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn cleanup_expired(&self, ttl: Duration) -> Result<usize, DomainError> {
        self.cleanup_expired(ttl)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::domain::model::value_objects::DaimyoId;
    use tempfile::tempdir;

    #[test]
    fn test_save_load_delete() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());
        let session_id = SessionId::new("test_user");

        let session = SessionData::new(
            session_id.clone(),
            Some(DaimyoId::new(1)),
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        );

        // 保存
        manager.save(&session).unwrap();

        // 読み込み
        let loaded = manager.load(&session_id).unwrap().unwrap();
        assert_eq!(loaded.session_id, session_id);
        assert_eq!(loaded.selected_daimyo_id, Some(DaimyoId::new(1)));

        // 削除
        assert!(manager.delete(&session_id).unwrap());
        assert!(manager.load(&session_id).unwrap().is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());

        let mut old_session = SessionData::new(
            SessionId::new("old_user"),
            None,
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        // 8日前に設定
        old_session.last_accessed_at = Utc::now() - Duration::days(8);
        manager.save(&old_session).unwrap();

        let new_session = SessionData::new(
            SessionId::new("new_user"),
            None,
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        manager.save(&new_session).unwrap();

        // 7日経過したものをクリーンアップ
        let deleted = manager.cleanup_expired(Duration::days(7)).unwrap();
        assert_eq!(deleted, 1);

        // old_user は消え、new_user は残っている
        assert!(manager.load(&SessionId::new("old_user")).unwrap().is_none());
        assert!(manager.load(&SessionId::new("new_user")).unwrap().is_some());
    }
}
