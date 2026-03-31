use crate::domain::error::DomainError;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{Amount, DaimyoId, IninFlag, KuniId, Rate};
use rand::Rng;

/// 国を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kuni {
    /// 国ID
    pub id: KuniId,
    /// 国名
    pub name: crate::domain::model::daimyo::DaimyoName, // 便宜上DaimyoNameを使い回すが、本来はKuniNameを作るべき。今回はDaimyoName(String)なので一旦これを使う。
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
        name: impl Into<String>,
        daimyo_id: DaimyoId,
        resource: Resource,
        stats: DevelopmentStats,
        inin: IninFlag,
    ) -> Self {
        Self {
            id,
            name: crate::domain::model::daimyo::DaimyoName(name.into()),
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
    pub fn add_resource(&mut self, kin: Amount, hei: Amount, kome: Amount) {
        self.resource.add(kin, hei, kome);
    }

    /// 資源を消費します。不足している場合は DomainError を返します。
    pub fn consume_resource(
        &mut self,
        kin: Amount,
        hei: Amount,
        kome: Amount,
    ) -> Result<(), DomainError> {
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
    // 各計算式は PRD.md の「資源計算式」章に基づいています。

    /// 米を売却します。
    /// 獲得：金 += 投入量 * (random(BIAS) + BIAS)
    pub fn sell_rice(&mut self, amount: Amount) -> Result<u32, DomainError> {
        let rng =
            rand::thread_rng().gen_range(0..crate::domain::model::value_objects::INTERNAL_SCALE);
        let multiplier = rng + crate::domain::model::value_objects::INTERNAL_SCALE;
        let gain =
            (amount.value() * multiplier) / crate::domain::model::value_objects::INTERNAL_SCALE;

        self.consume_resource(Amount::new(0), Amount::new(0), amount)?;
        self.add_resource(Amount::new(gain), Amount::new(0), Amount::new(0));
        Ok(gain / crate::domain::model::value_objects::INTERNAL_SCALE) // 表示単位の獲得額を返す
    }

    /// 米を購入します。
    /// 消費：金 -= 投入量 * (random(BIAS) + BIAS)
    pub fn buy_rice(&mut self, amount: Amount) -> Result<u32, DomainError> {
        let rng =
            rand::thread_rng().gen_range(0..crate::domain::model::value_objects::INTERNAL_SCALE);
        let multiplier = rng + crate::domain::model::value_objects::INTERNAL_SCALE;
        let cost =
            (amount.value() * multiplier) / crate::domain::model::value_objects::INTERNAL_SCALE;

        self.consume_resource(Amount::new(cost), Amount::new(0), Amount::new(0))?;
        self.add_resource(Amount::new(0), Amount::new(0), amount);
        Ok(cost / crate::domain::model::value_objects::INTERNAL_SCALE) // 表示単位の支払額を返す
    }

    /// 開墾を行い、石高を上昇させます。
    /// 獲得：石高 += 投入量 * (45 + random(10))
    pub fn develop_land(&mut self, investment: Amount) -> Result<u32, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = investment.value() * multiplier;

        self.consume_resource(investment, Amount::new(0), Amount::new(0))?;
        self.stats.kokudaka = self.stats.kokudaka.add(Amount::new(gain));
        Ok(gain / crate::domain::model::value_objects::INTERNAL_SCALE) // 表示単位の上昇量を返す
    }

    /// 町造りを行い、町ランクを上昇させます。
    /// 獲得：町 += 投入量 * (45 + random(10))
    pub fn build_town(&mut self, investment: Amount) -> Result<u32, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=54);
        let gain = investment.value() * multiplier;

        self.consume_resource(investment, Amount::new(0), Amount::new(0))?;
        self.stats.machi = self.stats.machi.add(Amount::new(gain));
        Ok(gain / crate::domain::model::value_objects::INTERNAL_SCALE) // 表示単位の上昇量を返す
    }

    /// 兵を徴募します。
    /// 消費：金 -= 投入量 * BIAS/2, 人口 -= 投入量 * BIAS, 忠誠度 -= 投入量 * BIAS/2
    /// 獲得：兵 += 投入量 * BIAS
    pub fn recruit_troops(&mut self, amount: Amount) -> Result<(), DomainError> {
        let cost = amount.value() / 2;
        let population_cost = amount.value();
        let tyu_loss = amount.value() / 2;

        if self.resource.jinko.value() < population_cost {
            return Err(DomainError::InsufficientResource("人口不足".to_string()));
        }

        self.consume_resource(Amount::new(cost), Amount::new(0), Amount::new(0))?;
        self.modify_jinko(-(population_cost as i32));
        self.modify_tyu(-(tyu_loss as i32));
        self.add_resource(Amount::new(0), amount, Amount::new(0));
        Ok(())
    }

    /// 兵を解雇します。
    /// 消費：兵 -= 投入量 * BIAS
    /// 獲得：忠誠度 += 投入量 * BIAS/2, 人口 += 投入量 * BIAS
    pub fn dismiss_troops(&mut self, amount: Amount) -> Result<(), DomainError> {
        self.consume_resource(Amount::new(0), amount, Amount::new(0))?;
        self.modify_jinko(amount.value() as i32);
        self.modify_tyu((amount.value() / 2) as i32);
        Ok(())
    }

    /// 施しを行い、忠誠度を上昇させます。
    /// 獲得：忠誠度 += 投入量 * (BIAS/2 + random(BIAS/2))
    pub fn give_charity(&mut self, amount: Amount) -> Result<u32, DomainError> {
        self.consume_resource(Amount::new(0), Amount::new(0), amount)?;

        let bias_half = crate::domain::model::value_objects::INTERNAL_SCALE / 2;
        let rng = rand::thread_rng().gen_range(0..bias_half);
        let tyu_gain = (amount.value() * (bias_half + rng))
            / crate::domain::model::value_objects::INTERNAL_SCALE;

        self.modify_tyu(tyu_gain as i32);
        Ok(tyu_gain) // 忠誠度上昇量を返す
    }
}
