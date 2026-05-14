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
        if my_hei.is_zero() {
            return 0.0;
        }
        let p = (enemy_hei.value() as f64 - my_hei.value() as f64) / my_hei.value() as f64;

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
            .map(|n| {
                Self::calculate_win_probability(n.resource.hei.mul_percent(50), target.resource.hei)
            }) // 敵が自分に勝つ確率
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
        let mut candidates = Vec::new();
        // 大名の性格 (軍事)
        let military_bias = daimyo.personality().military_bias();

        for neighbor in neighbors {
            if neighbor.daimyo_id == daimyo.id {
                continue;
            }

            // 相手の兵力より自分の兵力が少ない場合は攻めない
            if kuni.resource.hei.mul_percent(80) < neighbor.resource.hei {
                continue;
            }

            let min_hei = neighbor.resource.hei;
            let max_hei = kuni.resource.hei.mul_percent(80);
            let mut hei_candidates = Vec::new();

            for i in 0..=10 {
                let current_hei = min_hei + (max_hei - min_hei).mul_percent(i * 10);

                let my_rest_hei = kuni.resource.hei - current_hei;

                let rest_kuni = kuni.clone().with_hei(my_rest_hei);

                // 自国が攻め取られる確率を計算
                let my_risk_prob = (Self::calculate_lose_probability_from_neighbors(
                    &rest_kuni,
                    neighbor_repo,
                    kuni_repo,
                )
                .await?
                    / military_bias)
                    .clamp(0.0, 1.0);
                let win_prob = Self::calculate_win_probability(current_hei, neighbor.resource.hei);
                let rest_hei = current_hei - neighbor.resource.hei;

                // 占領後の状態をシミュレート（兵力は残存兵力、大名は自分）
                let win_kuni = neighbor.clone().with_hei(rest_hei).with_daimyo(daimyo.id);
                let risk_prob = (0.5
                    * Self::calculate_lose_probability_from_neighbors(
                        &win_kuni,
                        neighbor_repo,
                        kuni_repo,
                    )
                    .await?
                    / military_bias)
                    .clamp(0.0, 1.0);

                hei_candidates.push((
                    current_hei,
                    win_prob - (1.0f64 - (1.0f64 - my_risk_prob) * (1.0f64 - risk_prob)),
                ));
            }

            let (hei_candidate, score) = hei_candidates
                .into_iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap();

            // 期待値スコア = 勝率 - (戦争元が攻め取られる確率 || 攻め取った後の国が攻め取られる確率)
            candidates.push((neighbor.id, hei_candidate, score));
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // 全ての .await ポイントが終わった後に RNG を初期化する（ThreadRngはSendではないため）
        let mut rng = rand::thread_rng();

        // 最もスコアが高いターゲットを選択
        let (target_id, hei_candidate, score) = candidates
            .into_iter()
            .max_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        // 最終的な出兵判断：スコアから出兵確率を動的に計算する
        let dice_roll = rng.gen_range(0.0..1.0_f64);

        if dice_roll < score {
            let invasion_kome = kuni.resource.kome.mul_percent(80).min(hei_candidate);

            Ok(Some(InvasionPlan {
                target_kuni_id: target_id,
                hei: hei_candidate,
                kome: invasion_kome,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::DomainError;
    use crate::domain::model::daimyo::Daimyo;
    use crate::domain::model::daimyo_personality::DaimyoPersonality;
    use crate::domain::model::resource::{DevelopmentStats, Resource};
    use crate::domain::model::value_objects::*;
    use crate::domain::repository::kuni_repository::KuniRepository;
    use crate::domain::repository::neighbor_repository::NeighborRepository;
    use std::collections::HashMap;

    // --- Mocks ---
    struct MockKuniRepo {
        kunis: HashMap<KuniId, Kuni>,
    }
    #[async_trait::async_trait]
    impl KuniRepository for MockKuniRepo {
        async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
            Ok(self.kunis.get(id).cloned())
        }
        async fn find_by_daimyo_id(&self, _id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
            Ok(vec![])
        }
        async fn save(&self, _kuni: &Kuni) -> Result<(), DomainError> {
            Ok(())
        }
        async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
            Ok(vec![])
        }
    }

    struct MockNeighborRepo {
        adjacents: HashMap<KuniId, Vec<KuniId>>,
    }
    impl NeighborRepository for MockNeighborRepo {
        fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
            self.adjacents.get(kuni_id).cloned().unwrap_or_default()
        }
        fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
            self.adjacents.get(a).is_some_and(|list| list.contains(b))
        }
    }

    fn create_test_kuni(id: u32, daimyo_id: u32, hei: u32) -> Kuni {
        Kuni::new(
            KuniId::new(id),
            format!("Country-{}", id),
            DaimyoId::new(daimyo_id),
            Resource {
                kin: Amount::new(10000),
                hei: Amount::new(hei),
                kome: Amount::new(10000),
                jinko: Amount::new(10000),
            },
            DevelopmentStats {
                kokudaka: Amount::new(500),
                machi: Amount::new(500),
                tyu: Rate::new(50),
            },
            IninFlag::new(false),
        )
    }

    #[tokio::test]
    async fn test_decide_invasion_prefers_weak_neighbor() {
        let service = WarDecisionService::new();
        let personality = DaimyoPersonality::new(1.0, 1.0, 1.0, 0.0).unwrap();
        let my_daimyo = Daimyo::new(DaimyoId::new(1), "MyDaimyo", personality);

        // 自分: 1000兵 (出兵に500兵使う想定)
        let my_kuni = create_test_kuni(1, 1, 1000);

        // 隣国: 100兵 (非常に弱い)
        let weak_neighbor = create_test_kuni(2, 2, 100);

        // モックデータ作成
        let mut kunis = HashMap::new();
        kunis.insert(weak_neighbor.id, weak_neighbor.clone());
        let kuni_repo = MockKuniRepo { kunis };

        let mut adjacents = HashMap::new();
        adjacents.insert(KuniId::new(1), vec![KuniId::new(2)]);
        adjacents.insert(KuniId::new(2), vec![KuniId::new(1)]);
        let neighbor_repo = MockNeighborRepo { adjacents };

        // 実行 (100回試行して統計的に安定させる)
        let mut invasion_count = 0;
        for _ in 0..100 {
            let plan = service
                .decide_invasion(
                    &my_daimyo,
                    &my_kuni,
                    std::slice::from_ref(&weak_neighbor),
                    &neighbor_repo,
                    &kuni_repo,
                )
                .await
                .unwrap();
            if plan.is_some() {
                invasion_count += 1;
            }
        }

        // 1000兵 vs 100兵なら勝率は約82%であり、リスクを引いたスコアは約0.63
        // 100回中40回以上は侵攻が選ばれるはず（統計的に極めて高い確率）
        assert!(
            invasion_count > 40,
            "Invasion count was only {}",
            invasion_count
        );
    }

    #[tokio::test]
    async fn test_decide_invasion_avoids_strong_neighbor() {
        let service = WarDecisionService::new();
        let personality = DaimyoPersonality::new(1.0, 1.0, 1.0, 0.0).unwrap();
        let my_daimyo = Daimyo::new(DaimyoId::new(1), "MyDaimyo", personality);

        // 自分: 500兵 (出兵に250兵使う想定)
        let my_kuni = create_test_kuni(1, 1, 500);

        // 隣国: 2000兵 (圧倒的に強い)
        let strong_neighbor = create_test_kuni(2, 2, 2000);

        // モックデータ作成
        let mut kunis = HashMap::new();
        kunis.insert(strong_neighbor.id, strong_neighbor.clone());
        let kuni_repo = MockKuniRepo { kunis };

        let mut adjacents = HashMap::new();
        adjacents.insert(KuniId::new(1), vec![KuniId::new(2)]);
        adjacents.insert(KuniId::new(2), vec![KuniId::new(1)]);
        let neighbor_repo = MockNeighborRepo { adjacents };

        // 実行
        let mut invasion_count = 0;
        for _ in 0..10 {
            let plan = service
                .decide_invasion(
                    &my_daimyo,
                    &my_kuni,
                    std::slice::from_ref(&strong_neighbor),
                    &neighbor_repo,
                    &kuni_repo,
                )
                .await
                .unwrap();
            if plan.is_some() {
                invasion_count += 1;
            }
        }

        // 圧倒的に強い敵には攻めないはず
        assert_eq!(invasion_count, 0);
    }

    #[tokio::test]
    async fn test_decide_invasion_prefers_safer_target() {
        let service = WarDecisionService::new();
        let personality = DaimyoPersonality::new(1.0, 1.0, 1.0, 0.0).unwrap();
        let my_daimyo = Daimyo::new(DaimyoId::new(1), "MyDaimyo", personality);

        // 自分: 1000兵
        let my_kuni = create_test_kuni(1, 1, 1000);

        // 隣国A: 200兵, 隣接国なし (安全)
        let safe_neighbor = create_test_kuni(2, 2, 200);
        // 隣国B: 200兵, 隣接国多数 (危険)
        let risky_neighbor = create_test_kuni(3, 3, 200);

        // モックデータ作成
        let mut kunis = HashMap::new();
        kunis.insert(safe_neighbor.id, safe_neighbor.clone());
        kunis.insert(risky_neighbor.id, risky_neighbor.clone());
        let kuni_repo = MockKuniRepo { kunis };

        let mut adjacents = HashMap::new();
        // 自分は A, B 両方に隣接
        adjacents.insert(KuniId::new(1), vec![KuniId::new(2), KuniId::new(3)]);
        // A は自分以外に隣接なし
        adjacents.insert(KuniId::new(2), vec![KuniId::new(1)]);
        // B は多数に隣接 (自分 + 5カ国)
        adjacents.insert(
            KuniId::new(3),
            vec![
                KuniId::new(1),
                KuniId::new(10),
                KuniId::new(11),
                KuniId::new(12),
                KuniId::new(13),
                KuniId::new(14),
            ],
        );
        let neighbor_repo = MockNeighborRepo { adjacents };

        // 実行
        let mut target_a_count = 0;
        let mut target_b_count = 0;
        for _ in 0..20 {
            let plan = service
                .decide_invasion(
                    &my_daimyo,
                    &my_kuni,
                    &[safe_neighbor.clone(), risky_neighbor.clone()],
                    &neighbor_repo,
                    &kuni_repo,
                )
                .await
                .unwrap();
            if let Some(p) = plan {
                if p.target_kuni_id == safe_neighbor.id {
                    target_a_count += 1;
                } else if p.target_kuni_id == risky_neighbor.id {
                    target_b_count += 1;
                }
            }
        }

        // どちらも兵力は同じだが、リスクが低い A が優先的に選ばれるはず
        // スコア計算で A の方が高くなるため、candidates.sort_by で A が先頭に来る
        assert!(target_a_count > 0);
        assert_eq!(target_b_count, 0);
    }

    #[tokio::test]
    async fn test_decide_invasion_considers_my_defense_risk() {
        let service = WarDecisionService::new();
        let personality = DaimyoPersonality::new(1.0, 1.0, 1.0, 0.0).unwrap();
        let my_daimyo = Daimyo::new(DaimyoId::new(1), "MyDaimyo", personality);

        // 自分: 1000兵
        let my_kuni = create_test_kuni(1, 1, 1000);

        // ターゲットB: 200兵 (弱いが、自分は背後リスクが高い)
        let weak_neighbor = create_test_kuni(2, 2, 200);

        // モックデータ作成 (自分に隣接する他の敵国 C, D, E, F)
        let mut kunis = HashMap::new();
        kunis.insert(weak_neighbor.id, weak_neighbor.clone());
        for i in 3..=6 {
            let enemy = create_test_kuni(i, i, 500); // 適度な強さの他国
            kunis.insert(enemy.id, enemy);
        }
        let kuni_repo = MockKuniRepo { kunis };

        let mut adjacents = HashMap::new();
        // 自分(1) は ターゲット(2) 以外に、3, 4, 5, 6 とも隣接している (合計5カ国)
        adjacents.insert(
            KuniId::new(1),
            vec![
                KuniId::new(2),
                KuniId::new(3),
                KuniId::new(4),
                KuniId::new(5),
                KuniId::new(6),
            ],
        );
        // ターゲット(2) は自分(1) とのみ隣接
        adjacents.insert(KuniId::new(2), vec![KuniId::new(1)]);
        let neighbor_repo = MockNeighborRepo { adjacents };

        // 実行
        let mut invasion_count = 0;
        for _ in 0..100 {
            let plan = service
                .decide_invasion(
                    &my_daimyo,
                    &my_kuni,
                    std::slice::from_ref(&weak_neighbor),
                    &neighbor_repo,
                    &kuni_repo,
                )
                .await
                .unwrap();
            if plan.is_some() {
                invasion_count += 1;
            }
        }

        // 背後リスク(my_risk_prob)が 5カ国 * 0.1 = 0.5 となり、
        // スコアは約 0.982 - (1.0 - 0.5 * 0.9) = 0.982 - 0.55 = 0.432 まで低下する。
        // (修正前は勝率が50%に固定されていたためスコアがマイナスになっていた)
        // 試行回数100回なら、60回未満になる確率が高い（かつ、リスクなしの約80回よりは明らかに低い）。
        assert!(
            invasion_count < 60,
            "Invasion count was too high ({}) despite high back-attack risk",
            invasion_count
        );
    }
}
