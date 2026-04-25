use crate::domain::error::DomainError;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{
    Amount, DaimyoId, DisplayAmount, IninFlag, KuniId, KuniName, Rate,
};
use rand::Rng;

/// 国を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kuni {
    /// 国ID
    pub id: KuniId,
    /// 国名
    pub name: KuniName,
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
            name: KuniName(name.into()),
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
    /// 獲得：金 += 投入量 * (random(10) + 10) / 100
    pub fn sell_rice(&mut self, amount: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let rng = rand::thread_rng().gen_range(10..=20);
        let internal_amount = amount.to_internal();
        let gain = internal_amount.mul_percent(rng);

        self.consume_resource(Amount::zero(), Amount::zero(), internal_amount)?;
        self.add_resource(gain, Amount::zero(), Amount::zero());
        Ok(gain.to_display())
    }

    /// 米を購入します。
    /// 消費：金 -= 投入量 * (random(10) + 10) / 100
    pub fn buy_rice(&mut self, amount: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let rng = rand::thread_rng().gen_range(10..=20);
        let internal_amount = amount.to_internal();
        let cost = internal_amount.mul_percent(rng);

        self.consume_resource(cost, Amount::zero(), Amount::zero())?;
        self.add_resource(Amount::zero(), Amount::zero(), internal_amount);
        Ok(cost.to_display())
    }

    /// 開墾を行い、石高を上昇させます。
    /// 獲得：石高 += 投入量 * (45 + random(10)) / 100
    pub fn develop_land(
        &mut self,
        investment: DisplayAmount,
    ) -> Result<DisplayAmount, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=55);
        let internal_investment = investment.to_internal();
        // 投入量（表示値）の45-54倍が内部値の上昇量となる
        let gain = internal_investment.mul_percent(multiplier);

        self.consume_resource(internal_investment, Amount::new(0), Amount::new(0))?;
        self.stats.kokudaka = self.stats.kokudaka.add(gain);
        Ok(gain.to_display())
    }

    /// 町造りを行い、町ランクを上昇させます。
    /// 獲得：町 += 投入量 * (45 + random(10)) / 100
    pub fn build_town(&mut self, investment: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=55);
        let internal_investment = investment.to_internal();
        let gain = internal_investment.mul_percent(multiplier);

        self.consume_resource(internal_investment, Amount::zero(), Amount::zero())?;
        self.stats.machi = self.stats.machi.add(gain);
        Ok(gain.to_display())
    }

    /// 兵を徴募します。
    /// 消費：金 -= 投入量 / 2, 人口 -= 投入量, 忠誠度 -= 投入量 / 2
    /// 獲得：兵 += 投入量
    pub fn recruit_troops(&mut self, amount: DisplayAmount) -> Result<(), DomainError> {
        let internal_amount = amount.to_internal();
        let cost = internal_amount.mul_percent(50);
        // 忠誠度の減少量は投入量（表示値）の半分
        let tyu_loss = internal_amount.to_display().value() / 2;

        if self.resource.jinko < internal_amount {
            return Err(DomainError::InsufficientResource("人口不足".to_string()));
        }

        self.consume_resource(cost, Amount::zero(), Amount::zero())?;
        self.modify_jinko(-internal_amount.as_i32());
        self.modify_tyu(-(tyu_loss as i32));
        self.add_resource(Amount::zero(), internal_amount, Amount::zero());
        Ok(())
    }

    /// 兵を解雇します。
    /// 消費：兵 -= 投入量
    /// 獲得：忠誠度 += 投入量 / 2, 人口 += 投入量
    pub fn dismiss_troops(&mut self, amount: DisplayAmount) -> Result<(), DomainError> {
        let internal_amount = amount.to_internal();
        let tyu_gain = internal_amount.to_display().value() / 2;

        self.consume_resource(Amount::zero(), internal_amount, Amount::zero())?;
        self.modify_jinko(internal_amount.as_i32());
        self.modify_tyu(tyu_gain as i32);
        Ok(())
    }

    /// 施しを行い、忠誠度を上昇させます。
    /// 獲得：忠誠度 += 投入量 * (50 + random(50)) / 100
    pub fn give_charity(&mut self, amount: DisplayAmount) -> Result<u32, DomainError> {
        let internal_amount = amount.to_internal();
        self.consume_resource(Amount::zero(), Amount::zero(), internal_amount)?;
        let before = self.stats.tyu.value();

        let multiplier: u32 = rand::thread_rng().gen_range(50..=100);
        let tyu_gain = internal_amount.mul_percent(multiplier);

        self.modify_tyu(tyu_gain.to_display().value() as i32);
        Ok(self.stats.tyu.value().saturating_sub(before))
    }
}
