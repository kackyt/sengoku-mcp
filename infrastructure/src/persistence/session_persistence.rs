use crate::persistence::view_token_record::{
    join_ticket_file_name, view_token_file_name, ViewTokenRecord, JOIN_TICKET_DIR, VIEW_TOKEN_DIR,
};
use chrono::{DateTime, Duration, Utc};
use engine::domain::error::DomainError;
use engine::domain::model::join_ticket::{JoinCode, JoinTicket};
pub use engine::domain::model::session::SessionData;
use engine::domain::model::value_objects::{SessionId, ViewToken};
use engine::domain::repository::join_ticket_repository::JoinTicketRepository;
use engine::domain::repository::session_repository::SessionRepository;
use engine::domain::repository::view_token_repository::ViewTokenRepository;
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

/// セッションIDから保存用のファイル名（`<安全な名前>.json`）を生成します。
///
/// 英数字・`_`・`-` 以外の文字は `_` に置換し、ディレクトリトラバーサルや
/// オブジェクトキーの階層化を防ぎます。ファイル保存・GCS保存の双方で共通利用します。
pub(crate) fn session_file_name(session_id: &SessionId) -> String {
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
    format!("{}.json", safe_name)
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
        self.storage_dir.join(session_file_name(session_id))
    }

    /// 一時ファイルへ書き込んでからリネームすることで、ファイルをアトミックに保存します
    fn write_atomic(target_path: &Path, content: &str) -> Result<(), SessionPersistenceError> {
        let tmp_path = target_path.with_extension("tmp");
        if let Err(error) =
            fs::write(&tmp_path, content).and_then(|()| fs::rename(&tmp_path, target_path))
        {
            // 保存失敗時は一時ファイルを可能な限り削除し、元のI/Oエラーを返します。
            let _ = fs::remove_file(&tmp_path);
            return Err(SessionPersistenceError::IoError(error));
        }
        Ok(())
    }

    /// セッションデータをファイルにアトミック保存します
    pub fn save(&self, data: &SessionData) -> Result<(), SessionPersistenceError> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(data)?;
        Self::write_atomic(&self.session_file_path(&data.session_id), &json)
    }

    /// 閲覧トークンの対応表ファイルのパスを取得します
    fn view_token_path(&self, token: &ViewToken) -> PathBuf {
        self.storage_dir
            .join(VIEW_TOKEN_DIR)
            .join(view_token_file_name(token))
    }

    /// 閲覧トークンとセッションIDの対応を保存します
    pub fn save_view_token(
        &self,
        token: &ViewToken,
        session_id: &SessionId,
    ) -> Result<(), SessionPersistenceError> {
        fs::create_dir_all(self.storage_dir.join(VIEW_TOKEN_DIR))?;
        let json = serde_json::to_string_pretty(&ViewTokenRecord::new(session_id))?;
        Self::write_atomic(&self.view_token_path(token), &json)
    }

    /// 閲覧トークンに対応するセッションIDを取得します
    pub fn find_view_token(
        &self,
        token: &ViewToken,
    ) -> Result<Option<SessionId>, SessionPersistenceError> {
        let path = self.view_token_path(token);
        if !path.exists() {
            return Ok(None);
        }
        let record: ViewTokenRecord = serde_json::from_str(&fs::read_to_string(path)?)?;
        Ok(Some(record.session_id))
    }

    /// 閲覧トークンを削除します
    pub fn delete_view_token(&self, token: &ViewToken) -> Result<bool, SessionPersistenceError> {
        Self::remove_if_exists(&self.view_token_path(token))
    }

    /// ファイルが存在すれば削除し、削除した場合は `true` を返します
    fn remove_if_exists(path: &Path) -> Result<bool, SessionPersistenceError> {
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 参加チケットの保存先ディレクトリ
    fn join_ticket_dir(&self) -> PathBuf {
        self.storage_dir.join(JOIN_TICKET_DIR)
    }

    /// 参加チケットを保存します
    pub fn save_join_ticket(&self, ticket: &JoinTicket) -> Result<(), SessionPersistenceError> {
        fs::create_dir_all(self.join_ticket_dir())?;
        let json = serde_json::to_string_pretty(ticket)?;
        let path = self
            .join_ticket_dir()
            .join(join_ticket_file_name(&ticket.code));
        Self::write_atomic(&path, &json)
    }

    /// 参加コードに対応するチケットを読み込みます
    pub fn find_join_ticket(
        &self,
        code: &JoinCode,
    ) -> Result<Option<JoinTicket>, SessionPersistenceError> {
        let path = self.join_ticket_dir().join(join_ticket_file_name(code));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
    }

    /// 参加チケットを削除します
    pub fn delete_join_ticket(&self, code: &JoinCode) -> Result<bool, SessionPersistenceError> {
        Self::remove_if_exists(&self.join_ticket_dir().join(join_ticket_file_name(code)))
    }

    /// 有効期限切れ（または読み込めない）参加チケットを削除し、削除件数を返します
    pub fn cleanup_expired_join_tickets(
        &self,
        now: DateTime<Utc>,
    ) -> Result<usize, SessionPersistenceError> {
        let dir = self.join_ticket_dir();
        if !dir.exists() {
            return Ok(0);
        }
        let mut deleted_count = 0;
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_file() || path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let expired = fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str::<JoinTicket>(&content).ok())
                .is_none_or(|ticket| ticket.is_expired(now));
            if expired && fs::remove_file(&path).is_ok() {
                deleted_count += 1;
            }
        }
        Ok(deleted_count)
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
        Self::remove_if_exists(&self.session_file_path(session_id))
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

#[async_trait::async_trait]
impl JoinTicketRepository for SessionPersistenceManager {
    async fn save_ticket(&self, ticket: &JoinTicket) -> Result<(), DomainError> {
        self.save_join_ticket(ticket)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn find_ticket(&self, code: &JoinCode) -> Result<Option<JoinTicket>, DomainError> {
        self.find_join_ticket(code)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn delete_ticket(&self, code: &JoinCode) -> Result<bool, DomainError> {
        self.delete_join_ticket(code)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn cleanup_expired_tickets(&self, now: DateTime<Utc>) -> Result<usize, DomainError> {
        self.cleanup_expired_join_tickets(now)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

#[async_trait::async_trait]
impl ViewTokenRepository for SessionPersistenceManager {
    async fn register(&self, token: &ViewToken, session_id: &SessionId) -> Result<(), DomainError> {
        self.save_view_token(token, session_id)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn find_session_id(&self, token: &ViewToken) -> Result<Option<SessionId>, DomainError> {
        self.find_view_token(token)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn revoke(&self, token: &ViewToken) -> Result<bool, DomainError> {
        self.delete_view_token(token)
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
        assert!(!manager
            .session_file_path(&session_id)
            .with_extension("tmp")
            .exists());

        // 読み込み
        let loaded = manager.load(&session_id).unwrap().unwrap();
        assert_eq!(loaded.session_id, session_id);
        assert_eq!(loaded.selected_daimyo_id, Some(DaimyoId::new(1)));

        // 削除
        assert!(manager.delete(&session_id).unwrap());
        assert!(manager.load(&session_id).unwrap().is_none());
    }

    #[test]
    fn test_save_removes_temporary_file_when_rename_fails() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());
        let session = SessionData::new(
            SessionId::new("rename_failure"),
            None,
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let target_path = manager.session_file_path(&session.session_id);
        let tmp_path = target_path.with_extension("tmp");

        // 保存先をディレクトリにして、書き込み後のリネームを確実に失敗させます。
        fs::create_dir(&target_path).unwrap();
        let marker_path = target_path.join("keep.txt");
        fs::write(&marker_path, "keep").unwrap();

        assert!(matches!(
            manager.save(&session),
            Err(SessionPersistenceError::IoError(_))
        ));
        assert!(!tmp_path.exists());
        assert_eq!(fs::read_to_string(&marker_path).unwrap(), "keep");
    }

    #[test]
    fn test_save_returns_write_error_without_removing_directory() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());
        let session = SessionData::new(
            SessionId::new("write_failure"),
            None,
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let target_path = manager.session_file_path(&session.session_id);
        let tmp_path = target_path.with_extension("tmp");

        // 一時ファイルのパスをディレクトリにして書き込みを失敗させます。
        // クリーンアップできなくても、元の書き込みエラーと既存データを保持します。
        fs::create_dir(&tmp_path).unwrap();
        let marker_path = tmp_path.join("keep.txt");
        fs::write(&marker_path, "keep").unwrap();
        let expected_error = fs::write(&tmp_path, "test").unwrap_err().kind();

        match manager.save(&session) {
            Err(SessionPersistenceError::IoError(error)) => {
                assert_eq!(error.kind(), expected_error);
            }
            result => panic!("書き込みのI/Oエラーを期待しました: {result:?}"),
        }
        assert!(!target_path.exists());
        assert_eq!(fs::read_to_string(&marker_path).unwrap(), "keep");
    }

    #[test]
    fn test_view_token_save_find_delete() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());
        let token = ViewToken::generate();
        let session_id = SessionId::new("chat/123");

        assert!(manager.find_view_token(&token).unwrap().is_none());
        manager.save_view_token(&token, &session_id).unwrap();
        assert_eq!(manager.find_view_token(&token).unwrap(), Some(session_id));

        // セッションの期限切れクリーンアップはトークンを削除しない
        assert_eq!(manager.cleanup_expired(Duration::zero()).unwrap(), 0);
        assert!(manager.find_view_token(&token).unwrap().is_some());

        assert!(manager.delete_view_token(&token).unwrap());
        assert!(!manager.delete_view_token(&token).unwrap());
        assert!(manager.find_view_token(&token).unwrap().is_none());
    }

    #[test]
    fn test_join_ticket_save_find_delete_cleanup() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());
        let ticket = JoinTicket::new(
            JoinCode::generate(),
            SessionId::new("web_1"),
            Duration::minutes(30),
        );
        let mut expired = JoinTicket::new(
            JoinCode::generate(),
            SessionId::new("web_2"),
            Duration::minutes(30),
        );
        expired.expires_at = Utc::now() - Duration::minutes(1);

        manager.save_join_ticket(&ticket).unwrap();
        manager.save_join_ticket(&expired).unwrap();
        assert_eq!(
            manager.find_join_ticket(&ticket.code).unwrap(),
            Some(ticket.clone())
        );

        // 期限切れのチケットだけが削除される
        assert_eq!(manager.cleanup_expired_join_tickets(Utc::now()).unwrap(), 1);
        assert!(manager.find_join_ticket(&expired.code).unwrap().is_none());

        assert!(manager.delete_join_ticket(&ticket.code).unwrap());
        assert!(manager.find_join_ticket(&ticket.code).unwrap().is_none());
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
