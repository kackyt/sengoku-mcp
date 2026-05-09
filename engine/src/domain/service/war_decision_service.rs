use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{Amount, KuniId};
use rand::Rng;

/// 出兵計画
pub struct InvasionPlan {
    pub target_kuni_id: KuniId,
    pub hei: Amount,
    pub kome: Amount,
}

/// CPU大名の出兵（戦争）に関する意思決定を行うドメインサービス
pub struct WarDecisionService;

impl Default for WarDecisionService {
    fn default() -> Self {
        Self::new()
    }
}

impl WarDecisionService {
    pub fn new() -> Self {
        Self
    }

    /// 出兵を検討するかどうかの閾値を計算します（百分率）
    pub fn calculate_attack_threshold(military_bias: f64) -> f64 {
        (80.0 / military_bias).clamp(60.0, 95.0)
    }

    /// 出兵の意思決定を行います。
    pub fn decide_invasion(
        &self,
        daimyo: &Daimyo,
        kuni: &Kuni,
        neighbors: &[Kuni],
    ) -> Option<InvasionPlan> {
        let personality = &daimyo.personality;
        let bias = personality.military_bias();
        let my_hei = kuni.resource.hei.value() as f64;

        let mut rng = rand::thread_rng();

        // 候補国を選出：必要兵力倍率に確率的ゆらぎを加え、毎回同じ判断にならないようにする
        // military_biasが高い大名ほど少ない兵力差でも攻めようとする傾向があるが、
        // 乱数係数によって状況次第では攻めないこともある
        let mut candidates = Vec::new();
        for neighbor in neighbors {
            let enemy_hei = neighbor.resource.hei.value() as f64;
            // 必要兵力倍率：基準値1.25にゆらぎ(±0.3)を加え、military_biasで楽観補正
            let noise_multiplier = rng.gen_range(-0.1..=0.1_f64);
            let required_hei = enemy_hei * (1.25 + noise_multiplier);

            if my_hei > required_hei {
                let win_prob = (my_hei / (my_hei + enemy_hei)).clamp(0.0, 1.0);
                candidates.push((neighbor.id, win_prob, neighbor));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // 最も期待値が高いターゲットを選択
        // スコアにも乱数ゆらぎを乗せ、毎回同じ相手を狙わないようにする
        candidates.sort_by(|a, b| {
            let value_a = (a.2.stats.kokudaka.value() + a.2.stats.machi.value()) as f64;
            let value_b = (b.2.stats.kokudaka.value() + b.2.stats.machi.value()) as f64;
            let jitter_a = rng.gen_range(0.8..=1.2_f64);
            let jitter_b = rng.gen_range(0.8..=1.2_f64);
            let score_a = a.1.powf(1.0 / bias) * value_a * jitter_a;
            let score_b = b.1.powf(1.0 / bias) * value_b * jitter_b;
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let (target_id, win_prob, _) = candidates[0];

        // 最終的な出兵判断：勝率とmilitary_biasから出兵確率を動的に計算する
        // 出兵確率 = (勝率 * military_bias * 100) に ±15% のゆらぎ
        // これにより同一状況でも出兵するか否かが毎回確定しない
        let base_attack_prob = (win_prob * bias * 100.0).clamp(30.0, 90.0);
        let attack_noise = rng.gen_range(-15.0..=15.0_f64);
        let final_attack_prob = (base_attack_prob + attack_noise).clamp(0.0, 100.0);
        let dice_roll = rng.gen_range(0.0..100.0_f64);
        let hei_ratio = rng.gen_range(0.5..=0.8_f64);

        if dice_roll < final_attack_prob {
            let my_kome = kuni.resource.kome.value() as f64;
            let invasion_hei = (my_hei * hei_ratio).min(my_kome);
            let invasion_kome = invasion_hei;

            Some(InvasionPlan {
                target_kuni_id: target_id,
                hei: Amount::new(invasion_hei as u32),
                kome: Amount::new(invasion_kome as u32),
            })
        } else {
            None
        }
    }
}
