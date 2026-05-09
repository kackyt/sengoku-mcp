use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{Amount, KuniId};
use crate::domain::service::kuni_service::KuniService;
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

    fn calculate_win_probability(my_hei: Amount, enemy_hei: Amount) -> f64 {
        let p = (enemy_hei - my_hei).value() as f64 / my_hei.value() as f64;

        // logistic sigmoid
        let gain = 5.0f64;

        1.0f64 / (1.0 + std::f64::consts::E.powf(p * gain))
    }

    async fn calculate_lose_probability_from_neighbors(
        target: &Kuni,
        neighbor_repo: &dyn crate::domain::repository::neighbor_repository::NeighborRepository,
        kuni_repo: &dyn crate::domain::repository::kuni_repository::KuniRepository,
    ) -> anyhow::Result<f64> {
        let neighbors =
            KuniService::get_neighbor_kunis(&target.id, neighbor_repo, kuni_repo).await?;

        let max_prob = neighbors
            .iter()
            .filter(|n| n.daimyo_id != target.daimyo_id) // 敵対勢力のみ
            .map(|n| Self::calculate_win_probability(n.resource.hei, target.resource.hei)) // 敵が自分に勝つ確率
            .fold(0.0, f64::max);

        Ok(max_prob)
    }

    /// 出兵の意思決定を行います。
    pub async fn decide_invasion(
        &self,
        daimyo: &Daimyo,
        kuni: &Kuni,
        neighbors: &[Kuni],
        neighbor_repo: &dyn crate::domain::repository::neighbor_repository::NeighborRepository,
        kuni_repo: &dyn crate::domain::repository::kuni_repository::KuniRepository,
    ) -> anyhow::Result<Option<InvasionPlan>> {
        let personality = &daimyo.personality;
        let bias = personality.military_bias();
        let my_hei = kuni.resource.hei.mul_percent(50);

        // 国に残す兵力
        let my_rest_hei = kuni.resource.hei - my_hei;

        let rest_kuni = kuni.clone().with_hei(my_rest_hei);

        // 自国が攻め取られる確率を計算
        let my_risk_prob =
            Self::calculate_lose_probability_from_neighbors(&rest_kuni, neighbor_repo, kuni_repo)
                .await?;

        // 勝利確率 - 攻め取られる確率を計算
        let mut candidates = Vec::new();
        for neighbor in neighbors {
            if neighbor.daimyo_id == daimyo.id {
                continue;
            }

            let win_prob = Self::calculate_win_probability(my_hei, neighbor.resource.hei);
            let rest_hei = my_hei - neighbor.resource.hei;

            // 占領後の状態をシミュレート（兵力は残存兵力、大名は自分）
            let win_kuni = neighbor.clone().with_hei(rest_hei).with_daimyo(daimyo.id);
            let risk_prob = Self::calculate_lose_probability_from_neighbors(
                &win_kuni,
                neighbor_repo,
                kuni_repo,
            )
            .await?;

            // 期待値スコア = 勝率 - (戦争元が攻め取られる確率 || 攻め取った後の国が攻め取られる確率)
            candidates.push((
                neighbor.id,
                win_prob - (1.0f64 - (1.0f64 - my_risk_prob) * (1.0f64 - risk_prob)),
            ));
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // 全ての .await ポイントが終わった後に RNG を初期化する（ThreadRngはSendではないため）
        let mut rng = rand::thread_rng();

        // 最もスコアが高いターゲットを選択
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (target_id, score) = candidates[0];

        // 最終的な出兵判断：スコアとmilitary_biasから出兵確率を動的に計算する
        let dice_roll = rng.gen_range(0.0..1.0_f64);
        let hei_ratio = rng.gen_range(50..=80);

        if dice_roll < score {
            let my_kome = kuni.resource.kome;
            let invasion_hei = my_hei.mul_percent(hei_ratio).min(my_kome);

            Ok(Some(InvasionPlan {
                target_kuni_id: target_id,
                hei: invasion_hei,
                kome: invasion_hei,
            }))
        } else {
            Ok(None)
        }
    }
}
