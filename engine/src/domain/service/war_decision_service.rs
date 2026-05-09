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

        let mut candidates = Vec::new();
        for neighbor in neighbors {
            let enemy_hei = neighbor.resource.hei.value() as f64;
            // 80%の兵力で勝てるか判定 (military_biasで楽観度調整)
            let required_hei = enemy_hei * 1.25 / bias;

            if my_hei > required_hei {
                let win_prob = (my_hei / (my_hei + enemy_hei)).clamp(0.0, 1.0);
                candidates.push((neighbor.id, win_prob, neighbor));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // 最も期待値が高いターゲットを選択
        candidates.sort_by(|a, b| {
            let value_a = (a.2.stats.kokudaka.value() + a.2.stats.machi.value()) as f64;
            let value_b = (b.2.stats.kokudaka.value() + b.2.stats.machi.value()) as f64;
            let score_a = a.1.powf(1.0 / bias) * value_a;
            let score_b = b.1.powf(1.0 / bias) * value_b;
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let (target_id, win_prob, _) = candidates[0];

        // 最終的な出兵判断
        let mut rng = rand::thread_rng();
        let base_chance = 70.0;
        let noise = rng.gen_range(-10.0..10.0);
        let final_chance = ((base_chance + noise) as f64).clamp(50.0, 80.0);

        if win_prob * 100.0 > (100.0 - (final_chance * bias)) {
            let my_kome = kuni.resource.kome.value() as f64;
            let invasion_hei = (my_hei * 0.8).min(my_kome);
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
