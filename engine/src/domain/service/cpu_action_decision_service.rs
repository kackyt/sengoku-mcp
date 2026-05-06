use crate::domain::model::{
    daimyo_personality::DaimyoPersonality,
    kuni::Kuni,
    value_objects::{Amount, DisplayAmount, KuniId, TurnNumber},
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
        target_kuni_id: Option<KuniId>,
    },
    Rest,
}

pub struct CpuActionDecisionService;

impl CpuActionDecisionService {
    pub fn decide(
        personality: &DaimyoPersonality,
        target_kuni: &Kuni,
        turn: TurnNumber,
        rng: &mut impl Rng,
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
        let mut max_slope = -f64::INFINITY;

        let mut reasoning_lines = Vec::new();

        for atype in action_types {
            let max_amt = Self::get_max_affordable(target_kuni, atype);
            if max_amt == 0 {
                continue;
            }

            let base_slope = Self::calculate_expected_slope(target_kuni, turn, atype, personality);
            
            // ランダム性の適用
            // randomness=0.2 の場合、期待値に +/- 1.0 程度のノイズを加える
            let noise = if personality.randomness > 0.0 {
                (rng.gen::<f64>() - 0.5) * personality.randomness * 10.0
            } else {
                0.0
            };
            
            let slope = base_slope + noise;

            reasoning_lines.push(format!("{}: {:.2} (base: {:.2})", atype, slope, base_slope));

            if slope > max_slope {
                max_slope = slope;
                best_atype = atype;
            }
        }

        if best_atype == "Rest" {
            return (
                CpuActionDecision::Rest,
                format!("現状維持が最適であると判断しました。解析結果: {}", reasoning_lines.join(", ")),
            );
        }

        // 投入量をランダム化 (性格の揺らぎも影響させる)
        // 0.3 〜 0.7 の範囲で基本ランダム、揺らぎが大きいほど極端な値が出やすい
        let max_amt = Self::get_max_affordable(target_kuni, best_atype);
        let rand_val: f64 = rng.gen();
        let rate = 0.3 + rand_val * 0.4; // 0.3 〜 0.7
        let mut optimal_val = ((max_amt as f64 * rate) as u32).max(1);

        // GiveCharity の場合はオーバーキルを防止
        if best_atype == "GiveCharity" {
            let current_tyu = target_kuni.stats.tyu.value();
            let needed_gain = 100_u32.saturating_sub(current_tyu);
            let needed_rice = (needed_gain * 4 / 3).max(1);
            optimal_val = optimal_val.min(needed_rice);
        }

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

    fn calculate_expected_slope(
        kuni: &Kuni,
        turn: TurnNumber,
        atype: &str,
        personality: &DaimyoPersonality,
    ) -> f64 {
        // 各要素の1単位(DisplayAmount:1)あたりの評価値(Slope)
        let spring_coef = Self::turns_to_coef(turn.turns_until_season(0)) as f64;
        let fall_coef = Self::turns_to_coef(turn.turns_until_season(2)) as f64;

        // 大名の性格バイアスを基本評価値に乗算
        let kin_slope = (Self::EVALUATE_KIN_COEF as f64) * personality.commerce_bias;
        let kome_slope = (Self::EVALUATE_KOME_COEF as f64) * personality.agriculture_bias;
        let hei_slope = (Self::EVALUATE_HEI_COEF as f64) * personality.military_bias;

        // 開発要素の金・米評価への影響勾配
        // 将来の長期的な収入（10年分程度）を見込む
        const INVESTMENT_HORIZON: f64 = 10.0;

        // 町1単位(100)は平均32%の金を春に生む
        let machi_unit_slope = 0.32 * kin_slope * (spring_coef / 100.0) * INVESTMENT_HORIZON;
        // 石高1単位(100)は平均32%の米を秋に生む
        let kokudaka_unit_slope = 0.32 * kome_slope * (fall_coef / 100.0) * INVESTMENT_HORIZON;
        // 人口1単位(100)は春に12%の金、秋に12%の米を生む
        let jinko_unit_slope = (0.12 * kin_slope * (spring_coef / 100.0) + 0.12 * kome_slope * (fall_coef / 100.0)) * INVESTMENT_HORIZON;

        // 忠誠度の重み
        let mut tyu_base_val = 4.0;
        let current_tyu = kuni.stats.tyu.value();

        // 忠誠度が安全圏（反乱リスクなし的50以上）なら評価を大幅に下げる
        if current_tyu >= 50 {
            tyu_base_val *= 0.25; // 1/4に
        }
        // さらに100に近い場合は微増させるメリットが少ないため減衰
        if current_tyu >= 80 {
            tyu_base_val *= 0.5;
        }

        let tyu_slope = (tyu_base_val * 0.3 * spring_coef) + (tyu_base_val * 0.2 * fall_coef);

        match atype {
            "DevelopLand" => {
                // コスト: 1金, 利得: 0.5石高
                (0.5 * kokudaka_unit_slope) - kin_slope
            }
            "BuildTown" => {
                // コスト: 1金, 利得: 0.5町
                (0.5 * machi_unit_slope) - kin_slope
            }
            "SellRice" => {
                // コスト: 1米, 利得: 0.8金(期待値)
                (0.8 * kin_slope) - kome_slope
            }
            "BuyRice" => {
                // コスト: 1金, 利得: 0.8米(期待値)
                (0.8 * kome_slope) - kin_slope
            }
            "Recruit" => {
                // コスト: 0.5金, 1人口, 0.5忠誠, 利得: 1兵
                hei_slope - (0.5 * kin_slope) - jinko_unit_slope - (0.5 * tyu_slope)
            }
            "Dismiss" => {
                // コスト: 1兵, 利得: 1人口 + 0.5忠誠
                // 忠誠度がすでに100なら利得はない
                let tyu_gain_slope = if kuni.stats.tyu.value() < 100 {
                    0.5 * tyu_slope
                } else {
                    0.0
                };
                -hei_slope + jinko_unit_slope + tyu_gain_slope
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

    const EVALUATE_HEI_COEF: u32 = 45;
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
        let mut kuni = create_test_kuni(1000, 0, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();
        let personality = DaimyoPersonality::default();

        let (decision, reasoning) = CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);

        match decision {
            CpuActionDecision::DevelopLand { amount, .. } => {
                // 投入量がリソースの半分（500）付近であることを確認
                assert!(amount.value() >= 300 && amount.value() <= 700);
            }
            _ => panic!("Expected DevelopLand, but got {:?}. Reasoning: {}", decision, reasoning),
        }
    }

    #[test]
    fn test_decide_rest_when_no_resources() {
        let kuni = create_test_kuni(0, 0, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();
        let personality = DaimyoPersonality::default();

        let (decision, reason) = CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);
        println!("Decision: {:?}, Reason: {}", decision, reason);

        assert!(matches!(decision, CpuActionDecision::Rest));
    }

    #[test]
    fn test_decide_sell_rice_when_low_kin_high_kome() {
        // 金が0で、他のアクションが不可能な場合に、SellRice（利得正）が選択されるか
        let kuni = create_test_kuni(0, 2000, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();
        let personality = DaimyoPersonality::default();

        // 忠誠度を100にしてGiveCharityの勾配を-infにする
        let mut kuni_max_tyu = kuni.clone();
        kuni_max_tyu.stats.tyu = Rate::new(100);

        let (decision, reasoning) =
            CpuActionDecisionService::decide(&personality, &kuni_max_tyu, turn, &mut rng);

        println!("Reasoning: {}", reasoning);
        // 米を売って金を稼ぐ判断をするはず
        assert!(matches!(decision, CpuActionDecision::SellRice { .. }));
    }

    #[test]
    fn test_decide_prioritize_recruit_or_build_town_over_charity_in_fall() {
        // 秋、忠誠度が高い(60)、金・米・人口が十分にある状態
        let kuni = create_test_kuni(1000, 1000, 100, 100);
        let mut kuni = kuni;
        kuni.stats.tyu = Rate::new(60);
        kuni.resource.hei = Amount::new(500); // 兵も少し持たせる
        
        let turn = TurnNumber::new(3); // 秋
        let mut rng = thread_rng();
        let personality = DaimyoPersonality::default();

        let (decision, reasoning) =
            CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);

        println!("Reasoning: {}", reasoning);
        
        // 修正前は GiveCharity (勾配900超) が選ばれていたが、
        // 修正後は BuildTown (勾配480) などが選ばれるはず。
        // (Recruitは兵の評価係数200だと人口評価360に負けて負の勾配になるため、BuildTownが有力)
        match decision {
            CpuActionDecision::BuildTown { .. } => {},
            CpuActionDecision::SellRice { .. } => {}, // 金がもっと少なければこれもあり
            CpuActionDecision::GiveCharity { .. } => panic!("Should NOT choose GiveCharity when loyalty is 60 and BuildTown is possible"),
            _ => panic!("Expected productive action, but got {:?}", decision),
        }
    }

    #[test]
    fn test_give_charity_overkill_prevention() {
        // 忠誠度が95で、米が大量にある状態
        let mut kuni = create_test_kuni(0, 1000, 100, 100);
        kuni.stats.tyu = Rate::new(95);
        
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();
        let personality = DaimyoPersonality::default();

        let (decision, reasoning) =
            CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);

        println!("Reasoning: {}", reasoning);

        if let CpuActionDecision::GiveCharity { amount, .. } = decision {
            // 忠誠度を5上げるのに必要な米は 5 / 0.75 = 6.66... -> 7〜8程度
            // 以前なら 1000/2 = 500 投じていたが、制限がかかっているはず
            assert!(amount.value() < 20, "Amount {} is too large for gaining 5 loyalty", amount.value());
        }
    }

    #[test]
    fn test_personality_bias_agriculture() {
        let kuni = create_test_kuni(1000, 0, 100, 100);
        
        let turn = TurnNumber::new(1); // 夏（秋の収穫に近い）
        let mut rng = thread_rng();
        
        // 農業バイアスを極端に高くする
        let personality = DaimyoPersonality::new(10.0, 0.1, 0.1, 0.0);

        let (decision, _) = CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);

        // 農業重視なら開墾(DevelopLand)を選ぶはず
        assert!(matches!(decision, CpuActionDecision::DevelopLand { .. }));
    }

    #[test]
    fn test_personality_bias_commerce() {
        let kuni = create_test_kuni(1000, 0, 100, 100);
        
        let turn = TurnNumber::new(4); // 冬（来春の収入に向けて）
        let mut rng = thread_rng();
        
        // 商業バイアスを極端に高くする
        let personality = DaimyoPersonality::new(0.1, 10.0, 0.1, 0.0);

        let (decision, reason) = CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);
        println!("Decision: {:?}, Reason: {}", decision, reason);

        // 商業重視なら町造り(BuildTown)を選ぶはず
        assert!(matches!(decision, CpuActionDecision::BuildTown { .. }));
    }

    #[test]
    fn test_personality_bias_military() {
        let kuni = create_test_kuni(1000, 1000, 100, 100);
        let turn = TurnNumber::new(1);
        let mut rng = thread_rng();
        
        // 軍事バイアスを極端に高くする
        let personality = DaimyoPersonality::new(0.1, 0.1, 10.0, 0.0);

        let (decision, _) = CpuActionDecisionService::decide(&personality, &kuni, turn, &mut rng);

        // 軍事重視なら徴募(Recruit)を選ぶはず
        assert!(matches!(decision, CpuActionDecision::Recruit { .. }));
    }
}
