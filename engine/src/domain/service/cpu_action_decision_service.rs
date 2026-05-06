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
    SellRice {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    BuyRice {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    Recruit {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    Dismiss {
        target_kuni_id: KuniId,
        amount: DisplayAmount,
    },
    GiveCharity {
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
        _daimyo_id: DaimyoId,
        target_kuni: &Kuni,
        turn: TurnNumber,
        _rng: &mut impl Rng,
    ) -> (CpuActionDecision, String) {
        let base_score = Self::evaluate_score(target_kuni, turn);

        let action_types = vec![
            "DevelopLand",
            "BuildTown",
            "SellRice",
            "BuyRice",
            "Recruit",
            "Dismiss",
            "GiveCharity",
        ];

        let mut best_atype = "Rest";
        let mut max_slope = 0.0;

        for atype in action_types {
            let max_amt = Self::get_max_affordable(target_kuni, atype);
            if max_amt == 0 {
                continue;
            }

            let slope = Self::calculate_expected_slope(target_kuni, turn, atype);
            if slope > max_slope {
                max_slope = slope;
                best_atype = atype;
            }
        }

        if best_atype == "Rest" {
            return (
                CpuActionDecision::Rest,
                "現状維持が最適であると判断しました".to_string(),
            );
        }

        // 投入量を最大可能量の半分（50%）に設定
        let max_amt = Self::get_max_affordable(target_kuni, best_atype);
        let optimal_val = (max_amt / 2).max(1); // 最低1は実行
        let amount = DisplayAmount::new(optimal_val);

        let decision = match best_atype {
            "DevelopLand" => CpuActionDecision::DevelopLand {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "BuildTown" => CpuActionDecision::BuildTown {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "SellRice" => CpuActionDecision::SellRice {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "BuyRice" => CpuActionDecision::BuyRice {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "Recruit" => CpuActionDecision::Recruit {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "Dismiss" => CpuActionDecision::Dismiss {
                target_kuni_id: target_kuni.id,
                amount,
            },
            "GiveCharity" => CpuActionDecision::GiveCharity {
                target_kuni_id: target_kuni.id,
                amount,
            },
            _ => CpuActionDecision::Rest,
        };

        // 予想スコアの計算（概算）
        let expected_gain = (max_slope * optimal_val as f64) as u32;
        let expected_score = base_score.value() + expected_gain;

        let reasoning = format!(
            "線形最適化により {} を選択しました (勾配: {:.2}, 投入量: {}, 基準: {}, 予想: {})",
            best_atype,
            max_slope,
            optimal_val,
            base_score.value(),
            expected_score
        );

        (decision, reasoning)
    }

    fn get_max_affordable(kuni: &Kuni, atype: &str) -> u32 {
        match atype {
            "DevelopLand" | "BuildTown" | "BuyRice" => kuni.resource.kin.to_display().value(),
            "SellRice" | "GiveCharity" => kuni.resource.kome.to_display().value(),
            "Recruit" => {
                let max_jinko = kuni.resource.jinko.to_display().value();
                let max_kin = kuni.resource.kin.to_display().value() * 2; // コスト0.5金を考慮
                max_jinko.min(max_kin)
            }
            "Dismiss" => kuni.resource.hei.to_display().value(),
            _ => 0,
        }
    }

    fn calculate_expected_slope(kuni: &Kuni, turn: TurnNumber, atype: &str) -> f64 {
        // 各要素の1単位(DisplayAmount:1)あたりの評価値(Slope)
        let spring_coef = Self::turns_to_coef(turn.turns_until_season(0)) as f64;
        let fall_coef = Self::turns_to_coef(turn.turns_until_season(2)) as f64;

        let kin_slope = Self::EVALUATE_KIN_COEF as f64;
        let kome_slope = Self::EVALUATE_KOME_COEF as f64;
        let hei_slope = Self::EVALUATE_HEI_COEF as f64;

        // 開発要素の金・米評価への影響勾配
        let machi_slope = 32.0 * 0.3 * spring_coef;
        let kokudaka_slope = 100.0 * 0.2 * fall_coef;
        let jinko_slope = (12.0 * 0.3 * spring_coef) + (12.0 * 0.2 * fall_coef);
        let tyu_slope = (40.0 * 0.3 * spring_coef) + (40.0 * 0.2 * fall_coef);

        match atype {
            "DevelopLand" => {
                // コスト: 1金, 利得: 0.5石高(期待値)
                (0.5 * kokudaka_slope) - kin_slope
            }
            "BuildTown" => {
                // コスト: 1金, 利得: 0.5町(期待値)
                (0.5 * machi_slope) - kin_slope
            }
            "SellRice" => {
                // コスト: 1米, 利得: 1.2金(期待値)
                (1.2 * kin_slope) - kome_slope
            }
            "BuyRice" => {
                // コスト: 1金, 利得: 0.85米(期待値)
                (0.85 * kome_slope) - kin_slope
            }
            "Recruit" => {
                // コスト: 0.5金 + 1人口 + 0.5忠誠, 利得: 1兵
                // 忠誠度がすでに0なら損失はない
                let tyu_loss_slope = if kuni.stats.tyu.value() > 0 {
                    0.5 * tyu_slope
                } else {
                    0.0
                };
                hei_slope - (0.5 * kin_slope) - jinko_slope - tyu_loss_slope
            }
            "Dismiss" => {
                // コスト: 1兵, 利得: 1人口 + 0.5忠誠
                // 忠誠度がすでに100なら利得はない
                let tyu_gain_slope = if kuni.stats.tyu.value() < 100 {
                    0.5 * tyu_slope
                } else {
                    0.0
                };
                -hei_slope + jinko_slope + tyu_gain_slope
            }
            "GiveCharity" => {
                // コスト: 1米, 利得: 0.75忠誠(期待値)
                if kuni.stats.tyu.value() >= 100 {
                    return -f64::INFINITY;
                }
                (0.75 * tyu_slope) - kome_slope
            }
            _ => 0.0,
        }
    }

    fn turns_to_coef(turns: u32) -> u32 {
        match turns {
            1 => 120, // 収穫・収入まであと1期（最高価値）
            2 => 100,
            3 => 50,
            _ => 0,
        }
    }

    const EVALUATE_HEI_COEF: u32 = 200;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::resource::{DevelopmentStats, Resource};
    use crate::domain::model::value_objects::*;
    use rand::thread_rng;

    fn create_test_kuni(kin: u32, kome: u32, kokudaka: u32, machi: u32) -> Kuni {
        Kuni::new(
            KuniId(1),
            "テスト国",
            DaimyoId(1),
            Resource {
                kin: DisplayAmount::new(kin).to_internal(),
                kome: DisplayAmount::new(kome).to_internal(),
                hei: Amount::zero(),
                jinko: DisplayAmount::new(1000).to_internal(),
            },
            DevelopmentStats {
                kokudaka: DisplayAmount::new(kokudaka).to_internal(),
                machi: DisplayAmount::new(machi).to_internal(),
                tyu: Rate::new(50),
            },
            IninFlag(false),
        )
    }

    #[test]
    fn test_decide_develop_land_when_high_kin() {
        let kuni = create_test_kuni(1000, 1000, 100, 100);
        // 秋（2）に近いターン（例えば1）に設定
        let turn = TurnNumber::new(1); // 0:春, 1:夏, 2:秋, 3:冬
        let mut rng = thread_rng();

        let (decision, reasoning) =
            CpuActionDecisionService::decide(kuni.daimyo_id, &kuni, turn, &mut rng);

        match decision {
            CpuActionDecision::DevelopLand { amount, .. } => {
                // 投入量がリソースの半分（500）付近であることを確認
                assert!(amount.value() >= 400 && amount.value() <= 600);
            }
            _ => panic!("Expected DevelopLand, but got {:?}", decision),
        }
        println!("Reasoning: {}", reasoning);
    }

    #[test]
    fn test_decide_rest_when_no_resources() {
        let kuni = create_test_kuni(0, 0, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();

        let (decision, _) = CpuActionDecisionService::decide(kuni.daimyo_id, &kuni, turn, &mut rng);

        assert!(matches!(decision, CpuActionDecision::Rest));
    }

    #[test]
    fn test_decide_sell_rice_when_low_kin_high_kome() {
        // 金が0で、他のアクションが不可能な場合に、SellRice（利得正）が選択されるか
        let kuni = create_test_kuni(0, 2000, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();

        // 忠誠度を100にしてGiveCharityの勾配を-infにする
        let mut kuni_max_tyu = kuni.clone();
        kuni_max_tyu.stats.tyu = Rate::new(100);

        let (decision, reasoning) =
            CpuActionDecisionService::decide(kuni_max_tyu.daimyo_id, &kuni_max_tyu, turn, &mut rng);

        println!("Reasoning: {}", reasoning);
        // 米を売って金を稼ぐ判断をするはず
        assert!(matches!(decision, CpuActionDecision::SellRice { .. }));
    }
}
