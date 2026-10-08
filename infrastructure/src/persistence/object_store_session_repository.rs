use crate::persistence::session_persistence::session_file_name;
use crate::persistence::view_token_record::{
    view_token_file_name, ViewTokenRecord, VIEW_TOKEN_DIR,
};
use chrono::{Duration, Utc};
use engine::domain::error::DomainError;
use engine::domain::model::session::SessionData;
use engine::domain::model::value_objects::{SessionId, ViewToken};
use engine::domain::repository::session_repository::SessionRepository;
use engine::domain::repository::view_token_repository::ViewTokenRepository;
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

/// オブジェクトストレージを用いたセッション永続化に関するエラー
#[derive(Debug, Error)]
pub enum ObjectStoreSessionError {
    #[error("オブジェクトストレージのエラー: {0}")]
    ObjectStore(#[from] object_store::Error),
    #[error("セッションデータのJSON変換エラー: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<ObjectStoreSessionError> for DomainError {
    fn from(e: ObjectStoreSessionError) -> Self {
        DomainError::InfrastructureError(e.to_string())
    }
}

/// オブジェクトストレージ（Google Cloud Storage 等）にセッションデータを
/// JSON オブジェクトとして保存するリポジトリ実装
///
/// オブジェクトキーは `<prefix>/<セッションID>.json` となります。
/// テストでは `object_store::memory::InMemory` を注入して動作確認できます。
#[derive(Debug, Clone)]
pub struct ObjectStoreSessionRepository {
    store: Arc<dyn ObjectStore>,
    prefix: Path,
}

impl ObjectStoreSessionRepository {
    /// 任意の ObjectStore 実装とキーのプレフィックスを指定して生成します
    pub fn new(store: Arc<dyn ObjectStore>, prefix: impl AsRef<str>) -> Self {
        Self {
            store,
            prefix: Path::from(prefix.as_ref()),
        }
    }

    /// Google Cloud Storage のバケットを保存先として生成します。
    ///
    /// 認証情報は `GOOGLE_SERVICE_ACCOUNT` / `GOOGLE_SERVICE_ACCOUNT_KEY` 等の環境変数、
    /// または Application Default Credentials（Cloud Run のメタデータサーバー等）から解決します。
    pub fn gcs(bucket: &str, prefix: &str) -> Result<Self, ObjectStoreSessionError> {
        let store = GoogleCloudStorageBuilder::from_env()
            .with_bucket_name(bucket)
            .build()?;
        Ok(Self::new(Arc::new(store), prefix))
    }

    /// セッションIDから保存先のオブジェクトキーを取得します
    fn object_path(&self, session_id: &SessionId) -> Path {
        self.prefix.clone().join(session_file_name(session_id))
    }

    /// 閲覧トークンの対応表のオブジェクトキーを取得します
    fn view_token_path(&self, token: &ViewToken) -> Path {
        self.prefix
            .clone()
            .join(VIEW_TOKEN_DIR)
            .join(view_token_file_name(token))
    }

    /// 値を JSON として保存します（オブジェクトの書き込みはアトミック）
    async fn put_json<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), ObjectStoreSessionError> {
        let json = serde_json::to_vec_pretty(value)?;
        self.store.put(path, PutPayload::from(json)).await?;
        Ok(())
    }

    /// JSON オブジェクトを読み込みます。存在しない場合は `None` を返します。
    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<Option<T>, ObjectStoreSessionError> {
        match self.store.get(path).await {
            Ok(result) => {
                let bytes = result.bytes().await?;
                Ok(Some(serde_json::from_slice(&bytes)?))
            }
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// オブジェクトを削除します。削除対象が存在した場合は `true` を返します。
    async fn delete_if_exists(&self, path: &Path) -> Result<bool, ObjectStoreSessionError> {
        // GCS の削除は存在しないキーでもエラーにならない実装があるため、事前に存在確認する
        match self.store.head(path).await {
            Ok(_) => {}
            Err(object_store::Error::NotFound { .. }) => return Ok(false),
            Err(e) => return Err(e.into()),
        }
        match self.store.delete(path).await {
            Ok(()) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// セッションデータを保存します
    pub async fn save_data(&self, data: &SessionData) -> Result<(), ObjectStoreSessionError> {
        self.put_json(&self.object_path(&data.session_id), data)
            .await
    }

    /// セッションデータを読み込みます。存在しない場合は `None` を返します。
    pub async fn load_data(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionData>, ObjectStoreSessionError> {
        self.get_json(&self.object_path(session_id)).await
    }

    /// セッションデータを削除します。削除対象が存在した場合は `true` を返します。
    pub async fn delete_data(
        &self,
        session_id: &SessionId,
    ) -> Result<bool, ObjectStoreSessionError> {
        self.delete_if_exists(&self.object_path(session_id)).await
    }

    /// 最終アクセスから `ttl` 以上経過したセッションを削除し、削除件数を返します
    pub async fn cleanup_expired_data(
        &self,
        ttl: Duration,
    ) -> Result<usize, ObjectStoreSessionError> {
        let now = Utc::now();
        // 直下のオブジェクトのみを対象とし、閲覧トークン等のサブプレフィックスは除外する
        let objects = self
            .store
            .list_with_delimiter(Some(&self.prefix))
            .await?
            .objects;
        let mut deleted_count = 0;

        for meta in objects {
            if meta.location.extension() != Some("json") {
                continue;
            }

            // JSON として読めればその最終アクセス日時、読めなければ更新日時で期限切れを判定する
            let last_accessed_at = match self.store.get(&meta.location).await {
                Ok(result) => match result.bytes().await {
                    Ok(bytes) => serde_json::from_slice::<SessionData>(&bytes)
                        .map(|data| data.last_accessed_at)
                        .unwrap_or(meta.last_modified),
                    Err(_) => meta.last_modified,
                },
                // 一覧取得後に他プロセスが削除した場合はスキップする
                Err(object_store::Error::NotFound { .. }) => continue,
                Err(e) => return Err(e.into()),
            };

            if (now - last_accessed_at) > ttl && self.store.delete(&meta.location).await.is_ok() {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}

#[async_trait::async_trait]
impl SessionRepository for ObjectStoreSessionRepository {
    async fn save(&self, data: &SessionData) -> Result<(), DomainError> {
        Ok(self.save_data(data).await?)
    }

    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionData>, DomainError> {
        Ok(self.load_data(session_id).await?)
    }

    async fn delete(&self, session_id: &SessionId) -> Result<bool, DomainError> {
        Ok(self.delete_data(session_id).await?)
    }

    async fn cleanup_expired(&self, ttl: Duration) -> Result<usize, DomainError> {
        Ok(self.cleanup_expired_data(ttl).await?)
    }
}

#[async_trait::async_trait]
impl ViewTokenRepository for ObjectStoreSessionRepository {
    async fn register(&self, token: &ViewToken, session_id: &SessionId) -> Result<(), DomainError> {
        let record = ViewTokenRecord::new(session_id);
        Ok(self.put_json(&self.view_token_path(token), &record).await?)
    }

    async fn find_session_id(&self, token: &ViewToken) -> Result<Option<SessionId>, DomainError> {
        let record: Option<ViewTokenRecord> = self.get_json(&self.view_token_path(token)).await?;
        Ok(record.map(|r| r.session_id))
    }

    async fn revoke(&self, token: &ViewToken) -> Result<bool, DomainError> {
        Ok(self.delete_if_exists(&self.view_token_path(token)).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::domain::model::value_objects::DaimyoId;
    use futures::TryStreamExt;
    use object_store::memory::InMemory;

    /// テスト用に空のセッションデータを生成します
    fn empty_session(id: &str) -> SessionData {
        SessionData::new(
            SessionId::new(id),
            Some(DaimyoId::new(1)),
            None,
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn in_memory_repo() -> (Arc<InMemory>, ObjectStoreSessionRepository) {
        let store = Arc::new(InMemory::new());
        let repo = ObjectStoreSessionRepository::new(store.clone(), "sessions");
        (store, repo)
    }

    #[tokio::test]
    async fn test_save_load_delete() {
        let (store, repo) = in_memory_repo();
        let session = empty_session("user/../1");

        repo.save(&session).await.unwrap();

        // キーは prefix 配下の1階層に収まり、危険な文字は置換されている
        let keys: Vec<_> = store
            .list(None)
            .map_ok(|m| m.location.to_string())
            .try_collect()
            .await
            .unwrap();
        assert_eq!(keys, vec!["sessions/user____1.json".to_string()]);

        let loaded = repo.load(&session.session_id).await.unwrap().unwrap();
        assert_eq!(loaded.session_id, session.session_id);
        assert_eq!(loaded.selected_daimyo_id, Some(DaimyoId::new(1)));

        assert!(repo.delete(&session.session_id).await.unwrap());
        assert!(!repo.delete(&session.session_id).await.unwrap());
        assert!(repo.load(&session.session_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_load_returns_error_for_broken_json() {
        let (store, repo) = in_memory_repo();
        store
            .put(
                &Path::from("sessions/broken.json"),
                PutPayload::from_static(b"{not json"),
            )
            .await
            .unwrap();

        assert!(matches!(
            repo.load(&SessionId::new("broken")).await,
            Err(DomainError::InfrastructureError(_))
        ));
    }

    #[tokio::test]
    async fn test_view_token_save_find_delete() {
        let (_, repo) = in_memory_repo();
        let token = ViewToken::generate();
        let session_id = SessionId::new("chat/123");

        assert!(repo.find_session_id(&token).await.unwrap().is_none());
        repo.register(&token, &session_id).await.unwrap();
        assert_eq!(
            repo.find_session_id(&token).await.unwrap(),
            Some(session_id)
        );

        // セッションの期限切れクリーンアップはトークンを削除しない
        assert_eq!(repo.cleanup_expired(Duration::zero()).await.unwrap(), 0);
        assert!(repo.find_session_id(&token).await.unwrap().is_some());

        assert!(repo.revoke(&token).await.unwrap());
        assert!(!repo.revoke(&token).await.unwrap());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let (store, repo) = in_memory_repo();

        let mut old_session = empty_session("old_user");
        // 8日前に設定
        old_session.last_accessed_at = Utc::now() - Duration::days(8);
        repo.save(&old_session).await.unwrap();
        repo.save(&empty_session("new_user")).await.unwrap();

        // prefix 外や JSON 以外のオブジェクトは削除対象外
        store
            .put(
                &Path::from("other/old.json"),
                PutPayload::from(serde_json::to_vec(&old_session).unwrap()),
            )
            .await
            .unwrap();
        store
            .put(
                &Path::from("sessions/readme.txt"),
                PutPayload::from_static(b"keep"),
            )
            .await
            .unwrap();

        let deleted = repo.cleanup_expired(Duration::days(7)).await.unwrap();
        assert_eq!(deleted, 1);

        assert!(repo
            .load(&SessionId::new("old_user"))
            .await
            .unwrap()
            .is_none());
        assert!(repo
            .load(&SessionId::new("new_user"))
            .await
            .unwrap()
            .is_some());
        assert!(store.head(&Path::from("other/old.json")).await.is_ok());
        assert!(store.head(&Path::from("sessions/readme.txt")).await.is_ok());
    }
}
