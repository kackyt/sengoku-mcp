use crate::domain::error::DomainError;
use crate::domain::model::battle::WarStatus;
use crate::domain::model::value_objects::KuniId;
use async_trait::async_trait;

/// 合戦状態（バトルの状態）を永続化するためのリポジトリ
#[async_trait]
pub trait BattleRepository: Send + Sync {
    /// 攻撃側の国IDをキーとして合戦状態を保存します
    async fn save(&self, status: &WarStatus) -> Result<(), DomainError>;

    /// 攻撃側の国IDに関連付けられた合戦状態を取得します
    async fn find_by_attacker(
        &self,
        attacker_id: &KuniId,
    ) -> Result<Option<WarStatus>, DomainError>;

    /// 合戦が終了した際、状態を削除します
    async fn delete_by_attacker(&self, attacker_id: &KuniId) -> Result<(), DomainError>;
}
