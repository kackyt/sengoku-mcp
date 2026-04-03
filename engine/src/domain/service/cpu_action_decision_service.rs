use crate::domain::model::{
    kuni::Kuni,
    value_objects::{Amount, DaimyoId, KuniId},
};
use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub enum CpuActionDecision {
    DevelopLand {
        target_kuni_id: KuniId,
        amount: Amount,
    },
    BuildTown {
        target_kuni_id: KuniId,
        amount: Amount,
    },
    Battle {
        attacker_id: DaimyoId,
        target_kuni_id: Option<KuniId>,
    },
    Rest,
}

pub struct CpuActionDecisionService;

impl CpuActionDecisionService {
    pub fn decide(daimyo_id: DaimyoId, kunis: &[Kuni], rng: &mut impl Rng) -> CpuActionDecision {
        if kunis.is_empty() {
            return CpuActionDecision::Rest;
        }

        let target_kuni = &kunis[0];
        let action = rng.gen_range(0..4);

        match action {
            0 => CpuActionDecision::DevelopLand {
                target_kuni_id: target_kuni.id,
                amount: Amount::new(100),
            },
            1 => CpuActionDecision::BuildTown {
                target_kuni_id: target_kuni.id,
                amount: Amount::new(100),
            },
            2 => CpuActionDecision::Battle {
                attacker_id: daimyo_id,
                target_kuni_id: None, // 現時点では攻撃対象を決定できないためNoneを設定
            },
            _ => CpuActionDecision::Rest,
        }
    }
}
