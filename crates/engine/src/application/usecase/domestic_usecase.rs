use crate::domain::{model::value_objects::KuniId, repository::kuni_repository::KuniRepository};
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)]
pub struct DomesticUseCase<R: KuniRepository> {
    kuni_repo: Arc<R>,
}

impl<R: KuniRepository> DomesticUseCase<R> {
    pub fn new(kuni_repo: Arc<R>) -> Self {
        Self { kuni_repo }
    }

    pub async fn sell_rice(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        let bias: u32 = rand::thread_rng().gen_range(10..=20); // 1.0 to 2.0 multiplier mapped as 10 to 20
        let gain = (amount * bias) / 10;

        kuni.consume_resource(0, 0, amount)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.add_resource(gain, 0, 0);

        self.kuni_repo.save(&kuni).await
    }

    pub async fn buy_rice(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        let bias: u32 = rand::thread_rng().gen_range(10..=20);
        let cost = (amount * bias) / 10;

        kuni.consume_resource(cost, 0, 0)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.add_resource(0, 0, amount);

        self.kuni_repo.save(&kuni).await
    }

    pub async fn develop_land(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = amount * multiplier;

        kuni.consume_resource(amount, 0, 0)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.stats.kokudaka = kuni
            .stats
            .kokudaka
            .add(crate::domain::model::value_objects::Amount::new(gain));

        self.kuni_repo.save(&kuni).await
    }

    pub async fn build_town(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = amount * multiplier;

        kuni.consume_resource(amount, 0, 0)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.stats.machi = kuni
            .stats
            .machi
            .add(crate::domain::model::value_objects::Amount::new(gain));

        self.kuni_repo.save(&kuni).await
    }

    pub async fn recruit(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        let cost = amount / 2;
        let population_cost = amount;

        if kuni.resource.jinko.value() < population_cost {
            return Err(anyhow::anyhow!("Insufficient population"));
        }

        kuni.consume_resource(cost, 0, 0)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.modify_jinko(-(population_cost as i32));
        kuni.modify_tyu(-((amount / 2) as i32));
        kuni.add_resource(0, amount, 0);

        self.kuni_repo.save(&kuni).await
    }

    pub async fn dismiss(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;

        kuni.consume_resource(0, amount, 0)
            .map_err(|e| anyhow::anyhow!(e))?;
        kuni.modify_jinko(amount as i32);
        kuni.modify_tyu((amount / 2) as i32);

        self.kuni_repo.save(&kuni).await
    }

    pub async fn give_charity(&self, kuni_id: KuniId, amount: u32) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;

        kuni.consume_resource(0, 0, amount)
            .map_err(|e| anyhow::anyhow!(e))?;

        let multiplier: u32 = rand::thread_rng().gen_range(5..=10); // 0.5 to 1.0 mapped as 5 to 10
        let tyu_gain = (amount * multiplier) / 10;
        kuni.modify_tyu(tyu_gain as i32);

        self.kuni_repo.save(&kuni).await
    }

    pub async fn transport(
        &self,
        from_kuni_id: KuniId,
        to_kuni_id: KuniId,
        kin: u32,
        hei: u32,
        kome: u32,
    ) -> Result<(), anyhow::Error> {
        let mut from_kuni = self
            .kuni_repo
            .find_by_id(&from_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Source Kuni not found"))?;
        let mut to_kuni = self
            .kuni_repo
            .find_by_id(&to_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target Kuni not found"))?;

        from_kuni
            .consume_resource(kin, hei, kome)
            .map_err(|e| anyhow::anyhow!(e))?;
        to_kuni.add_resource(kin, hei, kome);

        self.kuni_repo.save(&from_kuni).await?;
        self.kuni_repo.save(&to_kuni).await
    }

    pub async fn set_delegation(
        &self,
        kuni_id: KuniId,
        delegate: bool,
    ) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Kuni not found"))?;
        kuni.set_inin(delegate);
        self.kuni_repo.save(&kuni).await
    }
}
