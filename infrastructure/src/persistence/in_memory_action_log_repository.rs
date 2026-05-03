use engine::domain::error::DomainError;
use engine::domain::model::action_log::{ActionLogCategory, ActionLogEntry, ActionLogVisibility};
use engine::domain::repository::action_log_repository::ActionLogRepository;
use std::collections::VecDeque;
use std::sync::RwLock;

/// インメモリのアクションログリポジトリ
pub struct InMemoryActionLogRepository {
    domestic_logs: RwLock<VecDeque<ActionLogEntry>>,
    war_logs: RwLock<VecDeque<ActionLogEntry>>,
    domestic_limit: usize,
    war_limit: usize,
}

impl InMemoryActionLogRepository {
    pub fn new() -> Self {
        Self {
            domestic_logs: RwLock::new(VecDeque::new()),
            war_logs: RwLock::new(VecDeque::new()),
            domestic_limit: 200,
            war_limit: 100,
        }
    }

    fn push_log(&self, category: ActionLogCategory, entry: ActionLogEntry) {
        match category {
            ActionLogCategory::Domestic => {
                let mut logs = self.domestic_logs.write().unwrap();
                logs.push_back(entry);
                while logs.len() > self.domestic_limit {
                    logs.pop_front();
                }
            }
            ActionLogCategory::War => {
                let mut logs = self.war_logs.write().unwrap();
                logs.push_back(entry);
                while logs.len() > self.war_limit {
                    logs.pop_front();
                }
            }
        }
    }
}

impl Default for InMemoryActionLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionLogRepository for InMemoryActionLogRepository {
    fn save(&self, entry: ActionLogEntry) -> Result<(), DomainError> {
        let category = entry.category;
        self.push_log(category, entry);
        Ok(())
    }

    fn find_visible(
        &self,
        category: ActionLogCategory,
        limit: usize,
    ) -> Result<Vec<ActionLogEntry>, DomainError> {
        let logs = match category {
            ActionLogCategory::Domestic => self.domestic_logs.read().unwrap(),
            ActionLogCategory::War => self.war_logs.read().unwrap(),
        };

        let visible_logs: Vec<ActionLogEntry> = logs
            .iter()
            .filter(|entry| {
                entry.visibility == ActionLogVisibility::Public
                    || entry.visibility == ActionLogVisibility::Player
            })
            .cloned()
            .collect();

        // 最新のlimit件を取得（末尾から）
        let start = visible_logs.len().saturating_sub(limit);
        Ok(visible_logs[start..].to_vec())
    }

    fn find_all(&self, category: ActionLogCategory) -> Result<Vec<ActionLogEntry>, DomainError> {
        let logs = match category {
            ActionLogCategory::Domestic => self.domestic_logs.read().unwrap(),
            ActionLogCategory::War => self.war_logs.read().unwrap(),
        };
        Ok(logs.clone().into())
    }

    fn clear(&self, category: ActionLogCategory) -> Result<(), DomainError> {
        match category {
            ActionLogCategory::Domestic => {
                self.domestic_logs.write().unwrap().clear();
            }
            ActionLogCategory::War => {
                self.war_logs.write().unwrap().clear();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::domain::model::value_objects::TurnNumber;

    fn create_entry(
        category: ActionLogCategory,
        visibility: ActionLogVisibility,
        msg: &str,
    ) -> ActionLogEntry {
        ActionLogEntry::new(
            category,
            visibility,
            TurnNumber::new(1),
            msg.to_string(),
            "".to_string(),
        )
    }

    #[test]
    fn test_save_and_find_visible() {
        let repo = InMemoryActionLogRepository::new();

        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Public,
            "pub",
        ))
        .unwrap();
        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            "player",
        ))
        .unwrap();
        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Internal,
            "internal",
        ))
        .unwrap();

        let visible = repo.find_visible(ActionLogCategory::Domestic, 10).unwrap();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].message, "pub");
        assert_eq!(visible[1].message, "player");
    }

    #[test]
    fn test_clear() {
        let repo = InMemoryActionLogRepository::new();
        repo.save(create_entry(
            ActionLogCategory::War,
            ActionLogVisibility::Public,
            "war_log",
        ))
        .unwrap();

        assert_eq!(repo.find_all(ActionLogCategory::War).unwrap().len(), 1);

        repo.clear(ActionLogCategory::War).unwrap();
        assert_eq!(repo.find_all(ActionLogCategory::War).unwrap().len(), 0);
    }

    #[test]
    fn test_limit() {
        let mut repo = InMemoryActionLogRepository::new();
        repo.domestic_limit = 2; // テスト用に上限を下げる

        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Public,
            "1",
        ))
        .unwrap();
        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Public,
            "2",
        ))
        .unwrap();
        repo.save(create_entry(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Public,
            "3",
        ))
        .unwrap();

        let all = repo.find_all(ActionLogCategory::Domestic).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].message, "2");
        assert_eq!(all[1].message, "3");
    }
}
