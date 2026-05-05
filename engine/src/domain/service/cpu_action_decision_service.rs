use crate::domain::model::{
    kuni::Kuni,
    value_objects::{Amount, DaimyoId, DisplayAmount, KuniId, TurnNumber},
};
use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub enum CpuActionDecision {
    DevelopLand {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    BuildTown {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    Battle {
        attacker_id: DaimyoId,
        target_kuni_id: Option<KuniId>,
    },
    Rest,
}

pub struct CpuActionDecisionService;

impl CpuActionDecisionService {
    pub fn decide(
        daimyo_id: DaimyoId,
        target_kuni: &Kuni,
        rng: &mut impl Rng,
    ) -> CpuActionDecision {
        let action = rng.gen_range(0..4);

        match action {
            0 => CpuActionDecision::DevelopLand {
                target_kuni_id: target_kuni.id,
                amount: DisplayAmount::new(1),
            },
            1 => CpuActionDecision::BuildTown {
                target_kuni_id: target_kuni.id,
                amount: DisplayAmount::new(1),
            },
            2 => CpuActionDecision::Battle {
                attacker_id: daimyo_id,
                target_kuni_id: None, // 現時点では攻撃対象を決定できないためNoneを設定
            },
            _ => CpuActionDecision::Rest,
        }
    }

    fn turns_to_coef(turns: u32) -> u32 {
        match turns {
            1 => 120,
            2 => 100,
            3 => 50,
            _ => 0,
        }
    }

    const EVALUATE_HEI_COEF: u32 = 50;
    const EVALUATE_KIN_COEF: u32 = 30;
    const EVALUATE_KOME_COEF: u32 = 20;

    pub fn evaluate_score(kuni: &Kuni, turn: TurnNumber) -> Amount {
        kuni.resource.hei.mul_percent(Self::EVALUATE_HEI_COEF)
            + kuni.resource.kin.mul_percent(Self::EVALUATE_KIN_COEF)
            + kuni.resource.kome.mul_percent(Self::EVALUATE_KOME_COEF)
            + (kuni.stats.machi.mul_percent(32)
                + kuni.resource.jinko.mul_percent(12)
                + kuni.stats.tyu.to_internal().mul_percent(4))
            .mul_percent(Self::EVALUATE_KIN_COEF)
            .mul_percent(Self::turns_to_coef(turn.turns_until_season(0)))
            + (kuni.stats.kokudaka.mul_percent(100)
                + kuni.resource.jinko.mul_percent(12)
                + kuni.stats.tyu.to_internal().mul_percent(4))
            .mul_percent(Self::EVALUATE_KOME_COEF)
            .mul_percent(Self::turns_to_coef(turn.turns_until_season(2)))
    }
}
