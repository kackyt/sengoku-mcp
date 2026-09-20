use chrono::{DateTime, Duration, Utc};
use engine::domain::model::action_log::ActionLogEntry;
use engine::domain::model::battle::WarStatus;
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::game_state::GameState;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::DaimyoId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 永続化対象のセッションデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// セッションの一意識別子
    pub session_id: String,
    /// 作成日時 (UTC)
    pub created_at: DateTime<Utc>,
    /// 最終アクセス日時 (UTC)
    pub last_accessed_at: DateTime<Utc>,
    /// 選択中の大名ID
    pub selected_daimyo_id: Option<DaimyoId>,
    /// ゲーム進行状態
    pub game_state: Option<GameState>,
    /// 全領地データ
    pub kunis: Vec<Kuni>,
    /// 全大名データ
    pub daimyos: Vec<Daimyo>,
    /// 進行中の合戦データ
    pub battles: Vec<WarStatus>,
    /// 行動ログ
    pub action_logs: Vec<ActionLogEntry>,
}

impl SessionData {
    /// 新規セッションデータを初期化します
    pub fn new(
        session_id: impl Into<String>,
        selected_daimyo_id: Option<DaimyoId>,
        game_state: Option<GameState>,
        kunis: Vec<Kuni>,
        daimyos: Vec<Daimyo>,
        battles: Vec<WarStatus>,
        action_logs: Vec<ActionLogEntry>,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            created_at: now,
            last_accessed_at: now,
            selected_daimyo_id,
            game_state,
            kunis,
            daimyos,
            battles,
            action_logs,
        }
    }

    /// アクセス日時を現在時刻に更新します
    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }
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

    /// セッションIDから安全なファイルパスを取得します
    fn session_file_path(&self, session_id: &str) -> PathBuf {
        // ディレクトリトラバーサル防止のため、ファイル名として安全な文字のみ残す
        let safe_name: String = session_id
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
    pub fn save(&self, data: &SessionData) -> Result<(), anyhow::Error> {
        self.ensure_dir()?;
        let target_path = self.session_file_path(&data.session_id);
        let tmp_path = target_path.with_extension("tmp");

        let json = serde_json::to_string_pretty(data)?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &target_path)?;

        Ok(())
    }

    /// セッションデータをファイルから読み込みます
    pub fn load(&self, session_id: &str) -> Result<Option<SessionData>, anyhow::Error> {
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let data: SessionData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// セッションファイルを削除します
    pub fn delete(&self, session_id: &str) -> Result<bool, anyhow::Error> {
        let path = self.session_file_path(session_id);
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 期限切れ（最終アクセスからttl以上経過）したセッションファイルを削除します
    /// 削除したファイル数を返します
    pub fn cleanup_expired(&self, ttl: Duration) -> Result<usize, anyhow::Error> {
        if !self.storage_dir.exists() {
            return Ok(0);
        }

        let now = Utc::now();
        let mut deleted_count = 0;

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                // セッションファイルを読み込んで判定
                let should_delete = match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<SessionData>(&content) {
                        Ok(data) => (now - data.last_accessed_at) > ttl,
                        Err(_) => {
                            // JSON破損ファイルの場合、ファイルのmtimeで判定
                            Self::is_file_expired(&path, ttl, now)
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_load_delete() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());

        let session = SessionData::new(
            "test_user",
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
        let loaded = manager.load("test_user").unwrap().unwrap();
        assert_eq!(loaded.session_id, "test_user");
        assert_eq!(loaded.selected_daimyo_id, Some(DaimyoId::new(1)));

        // 削除
        assert!(manager.delete("test_user").unwrap());
        assert!(manager.load("test_user").unwrap().is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let dir = tempdir().unwrap();
        let manager = SessionPersistenceManager::new(dir.path());

        let mut old_session =
            SessionData::new("old_user", None, None, vec![], vec![], vec![], vec![]);
        // 8日前に設定
        old_session.last_accessed_at = Utc::now() - Duration::days(8);
        manager.save(&old_session).unwrap();

        let new_session = SessionData::new("new_user", None, None, vec![], vec![], vec![], vec![]);
        manager.save(&new_session).unwrap();

        // 7日経過したものをクリーンアップ
        let deleted = manager.cleanup_expired(Duration::days(7)).unwrap();
        assert_eq!(deleted, 1);

        // old_user は消え、new_user は残っている
        assert!(manager.load("old_user").unwrap().is_none());
        assert!(manager.load("new_user").unwrap().is_some());
    }
}
