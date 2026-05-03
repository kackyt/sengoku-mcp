use crate::domain::error::DomainError;
use crate::domain::model::battle::ArmyStatus;
use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{
    Amount, DaimyoId, DisplayAmount, IninFlag, KuniId, KuniName, Rate,
};
use rand::Rng;

/// 割合減少の対象リソースを指定するセレクタ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSelector {
    /// 人口
    Jinko,
    /// 兵力
    Hei,
    /// 備蓄米
    Kome,
    /// 忠誠度（Rateとして管理）
    Tyu,
    /// 石高
    Kokudaka,
    /// 町
    Machi,
}

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

    /// 資源を消費します。不足している場合は DomainError を返します。
    pub fn consume_resource(
        &mut self,
        kin: Amount,
        hei: Amount,
        kome: Amount,
        jinko: Amount,
    ) -> Result<(), DomainError> {
        self.resource.consume(kin, hei, kome, jinko)
    }

    // --- 出陣・占領・防衛成功 (Battle関連) ---

    /// 出陣（必要な兵力と兵糧の保有検証と消費をカプセル化）
    pub fn dispatch_army(&mut self, hei: Amount, kome: Amount) -> Result<ArmyStatus, DomainError> {
        self.consume_resource(Amount::zero(), hei, kome, Amount::zero())?;
        Ok(ArmyStatus {
            kuni_id: self.id,
            hei,
            kome,
            morale: self.stats.tyu, // 出陣時の士気は忠誠度を引き継ぐ
        })
    }

    /// 他国を占領した時の事後処理
    /// （surviving_attacker はすでに敗者のリソースを合算済みの状態）
    pub fn occupy(&mut self, new_daimyo: DaimyoId, surviving_attacker: &ArmyStatus) {
        self.daimyo_id = new_daimyo;
        // 占領地に合算済みの軍勢（兵・兵糧）を配置
        self.resource.hei = surviving_attacker.hei;
        self.resource.kome = surviving_attacker.kome;
    }

    /// 防衛成功時の事後処理
    pub fn survive_defense(&mut self, surviving_defender: &ArmyStatus) {
        self.resource.hei = surviving_defender.hei;
        self.resource.kome = surviving_defender.kome;
        // 防衛後の忠誠度は、防衛軍の士気に置き換わる（または調整）
        self.stats.tyu = surviving_defender.morale;
    }

    // --- 内政ロジック (原子的な更新) ---
    // 各計算式は PRD.md の「資源計算式」章に基づいています。

    /// 米を売却します。
    pub fn sell_rice(&mut self, amount: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let rng = rand::thread_rng().gen_range(90..=150);
        let internal_amount = amount.to_internal();
        let gain = internal_amount.mul_percent(rng);

        self.consume_resource(
            Amount::zero(),
            Amount::zero(),
            internal_amount,
            Amount::zero(),
        )?;
        self.resource.kin += gain;
        Ok(gain.to_display())
    }

    /// 米を購入します。
    pub fn buy_rice(&mut self, amount: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let rng = rand::thread_rng().gen_range(70..=100);
        let internal_amount = amount.to_internal();
        let gain = internal_amount.mul_percent(rng);

        self.consume_resource(
            internal_amount,
            Amount::zero(),
            Amount::zero(),
            Amount::zero(),
        )?;
        self.resource.kome += gain;
        Ok(gain.to_display())
    }

    /// 開墾を行い、石高を上昇させます。
    pub fn develop_land(
        &mut self,
        investment: DisplayAmount,
    ) -> Result<DisplayAmount, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=55);
        let internal_investment = investment.to_internal();
        let gain = internal_investment.mul_percent(multiplier);

        self.consume_resource(
            internal_investment,
            Amount::zero(),
            Amount::zero(),
            Amount::zero(),
        )?;
        self.stats.kokudaka += gain;
        Ok(gain.to_display())
    }

    /// 町造りを行い、町ランクを上昇させます。
    pub fn build_town(&mut self, investment: DisplayAmount) -> Result<DisplayAmount, DomainError> {
        let multiplier: u32 = rand::thread_rng().gen_range(45..=55);
        let internal_investment = investment.to_internal();
        let gain = internal_investment.mul_percent(multiplier);

        self.consume_resource(
            internal_investment,
            Amount::zero(),
            Amount::zero(),
            Amount::zero(),
        )?;
        self.stats.machi += gain;
        Ok(gain.to_display())
    }

    /// 兵を徴募します。
    pub fn recruit_troops(&mut self, amount: DisplayAmount) -> Result<(), DomainError> {
        let internal_amount = amount.to_internal();
        let cost = internal_amount.mul_percent(50);
        let tyu_loss = amount.value() / 2;

        self.consume_resource(cost, Amount::zero(), Amount::zero(), internal_amount)?;
        self.stats.tyu -= Rate::new(tyu_loss);
        self.resource.hei += internal_amount;
        Ok(())
    }

    /// 兵を解雇します。
    pub fn dismiss_troops(&mut self, amount: DisplayAmount) -> Result<(), DomainError> {
        let internal_amount = amount.to_internal();
        let tyu_gain = amount.value() / 2;

        self.consume_resource(
            Amount::zero(),
            internal_amount,
            Amount::zero(),
            Amount::zero(),
        )?;
        self.resource.jinko += internal_amount;
        self.stats.tyu += Rate::new(tyu_gain);
        Ok(())
    }

    /// 施しを行い、忠誠度を上昇させます。
    pub fn give_charity(&mut self, amount: DisplayAmount) -> Result<u32, DomainError> {
        let internal_amount = amount.to_internal();
        self.consume_resource(
            Amount::zero(),
            Amount::zero(),
            internal_amount,
            Amount::zero(),
        )?;
        let before = self.stats.tyu.value();

        let multiplier: u32 = rand::thread_rng().gen_range(50..=100);
        let tyu_gain = amount.value() * multiplier / 100;

        self.stats.tyu += Rate::new(tyu_gain);
        Ok(self.stats.tyu.value().saturating_sub(before))
    }

    /// 指定したリソースをパーセンテージ（整数）で減少させます。
    /// 実際に減少した量を Amount 型で返します（忠誠度の場合は0を返します）。
    /// saturating_sub を使用するため、値が負になる心配はありません。
    pub fn apply_percentage_loss(&mut self, selector: ResourceSelector, percent: u32) -> Amount {
        match selector {
            ResourceSelector::Jinko => {
                let loss = self.resource.jinko.mul_percent(percent);
                self.resource.jinko -= loss;
                loss
            }
            ResourceSelector::Hei => {
                let loss = self.resource.hei.mul_percent(percent);
                self.resource.hei -= loss;
                loss
            }
            ResourceSelector::Kome => {
                let loss = self.resource.kome.mul_percent(percent);
                self.resource.kome -= loss;
                loss
            }
            ResourceSelector::Tyu => {
                // 忠誠度は Rate 型なのでパーセンテージ減少を個別に計算する
                let loss = self.stats.tyu.value() * percent / 100;
                self.stats.tyu -= Rate::new(loss);
                // 忠誠度の損失はAmount型で表現できないため0を返す
                Amount::zero()
            }
            ResourceSelector::Kokudaka => {
                let loss = self.stats.kokudaka.mul_percent(percent);
                self.stats.kokudaka -= loss;
                loss
            }
            ResourceSelector::Machi => {
                let loss = self.stats.machi.mul_percent(percent);
                self.stats.machi -= loss;
                loss
            }
        }
    }
}
