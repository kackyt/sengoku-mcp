use crate::domain::error::DomainError;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{Amount, DaimyoId, IninFlag, KuniId, Rate};
use rand::Rng;

/// 国を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kuni {
    /// 国ID
    pub id: KuniId,
    /// 支配している大名のID
    pub daimyo_id: DaimyoId,
    /// 資源（金、兵、米、人口）
    pub resource: Resource,
    /// 開発ステータス（石高、町、忠誠度）
    pub stats: DevelopmentStats,
    /// 委任フラグ
    pub inin: IninFlag,
}

impl Kuni {
    /// 新しい国を作成します
    pub fn new(
        id: KuniId,
        daimyo_id: DaimyoId,
        resource: Resource,
        stats: DevelopmentStats,
        inin: IninFlag,
    ) -> Self {
        Self {
            id,
            daimyo_id,
            resource,
            stats,
            inin,
        }
    }

    /// 支配大名を変更します
    pub fn set_daimyo_id(&mut self, daimyo_id: DaimyoId) {
        self.daimyo_id = daimyo_id;
    }

    /// 委任状態を設定します
    pub fn set_inin(&mut self, inin: IninFlag) {
        self.inin = inin;
    }

    /// 資源を追加します
    pub fn add_resource(&mut self, kin: u32, hei: u32, kome: u32) {
        self.resource.add(kin, hei, kome);
    }

    /// 資源を消費します。不足している場合は DomainError を返します。
    pub fn consume_resource(&mut self, kin: u32, hei: u32, kome: u32) -> Result<(), DomainError> {
        self.resource
            .consume(kin, hei, kome)
            .map_err(|e| DomainError::InsufficientResource(e.to_string()))
    }

    /// 人口を増減させます
    pub fn modify_jinko(&mut self, delta: i32) {
        let current = self.resource.jinko.value() as i32;
        let next = (current + delta).max(0) as u32;
        self.resource.jinko = Amount::new(next);
    }

    /// 忠誠度を増減させます
    pub fn modify_tyu(&mut self, delta: i32) {
        let current = self.stats.tyu.value() as i32;
        let next = (current + delta).clamp(0, 100) as u32;
        self.stats.tyu = Rate::new(next);
    }

    // --- 内政ロジック (Usecaseから移譲) ---

    /// 米を売却します。価格はランダムに変動します。
    pub fn sell_rice(&mut self, amount: u32) -> Result<(), DomainError> {
        let bias: u32 = rand::thread_rng().gen_range(10..=20); // 1.0 to 2.0 倍 (10-20で表現)
        let gain = (amount * bias) / 10;

        self.consume_resource(0, 0, amount)?;
        self.add_resource(gain, 0, 0);
        Ok(())
    }

    /// 米を購入します。価格はランダムに変動します。
    pub fn buy_rice(&mut self, amount: u32) -> Result<(), DomainError> {
        let bias: u32 = rand::thread_rng().gen_range(10..=20);
        let cost = (amount * bias) / 10;

        self.consume_resource(cost, 0, 0)?;
        self.add_resource(0, 0, amount);
        Ok(())
    }

    /// 開墾を行い、石高を上昇させます。
    pub fn develop_land(&mut self, investment: u32) -> Result<(), DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = investment * multiplier;

        self.consume_resource(investment, 0, 0)?;
        self.stats.kokudaka = self.stats.kokudaka.add(Amount::new(gain));
        Ok(())
    }

    /// 町作りを行い、町ランクを上昇させます。
    pub fn build_town(&mut self, investment: u32) -> Result<(), DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = investment * multiplier;

        self.consume_resource(investment, 0, 0)?;
        self.stats.machi = self.stats.machi.add(Amount::new(gain));
        Ok(())
    }

    /// 兵を徴募します。金と人口を消費し、忠誠度が低下します。
    pub fn recruit_troops(&mut self, amount: u32) -> Result<(), DomainError> {
        let cost = amount / 2;
        let population_cost = amount;

        if self.resource.jinko.value() < population_cost {
            return Err(DomainError::InsufficientResource("人口不足".to_string()));
        }

        self.consume_resource(cost, 0, 0)?;
        self.modify_jinko(-(population_cost as i32));
        self.modify_tyu(-((amount / 2) as i32));
        self.add_resource(0, amount, 0);
        Ok(())
    }

    /// 兵を解雇します。人口に戻り、忠誠度が上昇します。
    pub fn dismiss_troops(&mut self, amount: u32) -> Result<(), DomainError> {
        self.consume_resource(0, amount, 0)?;
        self.modify_jinko(amount as i32);
        self.modify_tyu((amount / 2) as i32);
        Ok(())
    }

    /// 施しを行い、忠誠度を上昇させます。
    pub fn give_charity(&mut self, amount: u32) -> Result<(), DomainError> {
        self.consume_resource(0, 0, amount)?;

        let multiplier: u32 = rand::thread_rng().gen_range(5..=10); // 0.5 to 1.0 倍
        let tyu_gain = (amount * multiplier) / 10;
        self.modify_tyu(tyu_gain as i32);
        Ok(())
    }
}
