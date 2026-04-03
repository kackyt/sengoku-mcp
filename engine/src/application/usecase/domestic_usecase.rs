use crate::domain::{
    error::DomainError,
    model::value_objects::{Amount, IninFlag, KuniId},
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
};
use std::sync::Arc;

/// 内政に関するユースケース
#[allow(dead_code)]
pub struct DomesticUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
}

impl DomesticUseCase {
    /// 新しい内政ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
        }
    }

    /// 米を売却します
    pub async fn sell_rice(&self, kuni_id: KuniId, amount: Amount) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.sell_rice(amount)?;

        self.kuni_repo.save(&kuni).await?;
        Ok(gain)
    }

    /// 米を購入します
    pub async fn buy_rice(&self, kuni_id: KuniId, amount: Amount) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let cost = kuni.buy_rice(amount)?;

        self.kuni_repo.save(&kuni).await?;
        Ok(cost)
    }

    /// 開墾を行います
    pub async fn develop_land(
        &self,
        kuni_id: KuniId,
        amount: Amount,
    ) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.develop_land(amount)?;

        self.kuni_repo.save(&kuni).await?;
        Ok(gain)
    }

    /// 町作りを行います
    pub async fn build_town(&self, kuni_id: KuniId, amount: Amount) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.build_town(amount)?;

        self.kuni_repo.save(&kuni).await?;
        Ok(gain)
    }

    /// 兵を徴募します
    pub async fn recruit(&self, kuni_id: KuniId, amount: Amount) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        kuni.recruit_troops(amount)?;

        self.kuni_repo.save(&kuni).await.map_err(|e| e.into())
    }

    /// 兵を解雇します
    pub async fn dismiss(&self, kuni_id: KuniId, amount: Amount) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        kuni.dismiss_troops(amount)?;

        self.kuni_repo.save(&kuni).await.map_err(|e| e.into())
    }

    /// 施しを行います
    pub async fn give_charity(
        &self,
        kuni_id: KuniId,
        amount: Amount,
    ) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.give_charity(amount)?;

        self.kuni_repo.save(&kuni).await?;
        Ok(gain)
    }

    /// 輸送を行います
    pub async fn transport(
        &self,
        from_kuni_id: KuniId,
        to_kuni_id: KuniId,
        kin: Amount,
        hei: Amount,
        kome: Amount,
    ) -> Result<(), anyhow::Error> {
        let mut from_kuni = self
            .kuni_repo
            .find_by_id(&from_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("送り元の国が見つかりません: {:?}", from_kuni_id))?;
        let mut to_kuni = self
            .kuni_repo
            .find_by_id(&to_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("送り先の国が見つかりません: {:?}", to_kuni_id))?;

        if !self.neighbor_repo.are_adjacent(&from_kuni_id, &to_kuni_id) {
            return Err(DomainError::NotAdjacent.into());
        }

        from_kuni.consume_resource(kin, hei, kome)?;
        to_kuni.add_resource(kin, hei, kome);

        self.kuni_repo.save(&from_kuni).await?;
        self.kuni_repo.save(&to_kuni).await.map_err(|e| e.into())
    }

    /// 委任状態を設定します
    pub async fn set_delegation(
        &self,
        kuni_id: KuniId,
        delegate: bool,
    ) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;
        kuni.set_inin(IninFlag::new(delegate));
        self.kuni_repo.save(&kuni).await.map_err(|e| e.into())
    }
}
