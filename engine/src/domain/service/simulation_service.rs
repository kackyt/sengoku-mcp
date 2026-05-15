use crate::domain::error::DomainError;
use crate::domain::model::{
    battle::{BattleSide, WarStatus},
    daimyo::Daimyo,
    kuni::Kuni,
    value_objects::{DaimyoId, KuniId, TurnNumber},
};
use crate::domain::repository::{
    kuni_repository::KuniRepository, neighbor_repository::NeighborRepository,
};
use crate::domain::service::{
    battle_service::BattleService, cpu_action_decision_service::CpuActionDecisionService,
    kuni_action_service::KuniActionService, seasonal_event_service::SeasonalEventService,
    war_decision_service::WarDecisionService,
};
use async_trait::async_trait;
use rand::Rng;
use std::collections::HashMap;

/// シミュレーションの結果のスナップショット
#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub turn: TurnNumber,
    pub kuni_states: Vec<Kuni>,
    /// このターンに発生した戦闘のログ（簡易版）
    pub logs: Vec<String>,
}

pub struct SimulationService;

impl SimulationService {
    /// 指定された大名と国のリストを用いて、指定ターン数分シミュレーションを実行します。
    pub async fn run_simulation(
        daimyos: &[Daimyo],
        initial_kunis: &[Kuni],
        neighbors: &HashMap<KuniId, Vec<KuniId>>,
        num_turns: u32,
        rng: &mut impl Rng,
    ) -> Result<Vec<SimulationSnapshot>, DomainError> {
        let mut kunis = initial_kunis.to_vec();
        let mut snapshots = Vec::new();
        let seasonal_service = SeasonalEventService::new();
        let war_decision_service = WarDecisionService::new();

        let daimyo_map: HashMap<_, _> = daimyos.iter().map(|d| (d.id, d)).collect();

        for t in 1..=num_turns {
            let turn = TurnNumber::new(t);
            let mut turn_logs = Vec::new();

            // 1. ターン開始時の季節イベント
            for kuni in kunis.iter_mut() {
                seasonal_service.process_start_turn_events(turn, kuni);
            }

            // 2. 各大名の行動
            // 戦争によって国の状態が変わるため、インデックスで管理
            let mut acted = vec![false; kunis.len()];

            for i in 0..kunis.len() {
                if acted[i] {
                    continue;
                }

                let kuni = &kunis[i];
                let daimyo = daimyo_map.get(&kuni.daimyo_id).ok_or_else(|| {
                    DomainError::NotFound(format!("Daimyo not found: {:?}", kuni.daimyo_id))
                })?;

                // 隣接国の情報を取得
                let neighbor_ids = neighbors.get(&kuni.id).cloned().unwrap_or_default();
                let neighbor_kunis: Vec<Kuni> = kunis
                    .iter()
                    .filter(|k| neighbor_ids.contains(&k.id))
                    .cloned()
                    .collect();

                // 出兵判断
                let kuni_repo = SimulationKuniRepo { kunis: &kunis };
                let neighbor_repo = SimulationNeighborRepo { neighbors };

                let invasion_plan = war_decision_service
                    .decide_invasion(daimyo, kuni, &neighbor_kunis, &neighbor_repo, &kuni_repo)
                    .await?;

                if let Some(plan) = invasion_plan {
                    // 戦争実行
                    let target_idx = kunis
                        .iter()
                        .position(|k| k.id == plan.target_kuni_id)
                        .ok_or_else(|| {
                            DomainError::NotFound(format!(
                                "Target kuni not found: {:?}",
                                plan.target_kuni_id
                            ))
                        })?;

                    let attacker_name = kuni.name.clone();
                    let target_name = kunis[target_idx].name.clone();

                    let defender_daimyo_id = kunis[target_idx].daimyo_id;
                    let defender_daimyo = daimyo_map.get(&defender_daimyo_id).ok_or_else(|| {
                        DomainError::NotFound(format!("Daimyo not found: {:?}", defender_daimyo_id))
                    })?;

                    let attacker_army = kunis[i].dispatch_army(plan.hei, plan.kome)?;

                    let target_hei = kunis[target_idx].resource.hei;
                    let target_kome = kunis[target_idx].resource.kome;
                    let defender_army = kunis[target_idx].dispatch_army(target_hei, target_kome)?;

                    let war_status = WarStatus {
                        attacker: attacker_army,
                        defender: defender_army,
                        winner: None,
                        advantage: crate::domain::model::battle::BattleAdvantage::Even,
                    };

                    let (final_status, _battle_turns) =
                        BattleService::auto_resolve(war_status, rng)?;

                    if final_status.winner == Some(BattleSide::Attacker) {
                        let attacker_daimyo_id = kunis[i].daimyo_id;
                        kunis[target_idx].occupy(attacker_daimyo_id, &final_status.attacker);
                        turn_logs.push(format!(
                            "【戦争】{}({}) が {}({}) を占領しました！",
                            daimyo.name.0, attacker_name.0, defender_daimyo.name.0, target_name.0
                        ));
                    } else {
                        kunis[target_idx].survive_defense(&final_status.defender);
                        turn_logs.push(format!(
                            "【戦争】{}({}) は {}({}) からの防衛に成功しました。",
                            daimyo.name.0, attacker_name.0, defender_daimyo.name.0, target_name.0
                        ));
                    }

                    acted[i] = true;
                } else {
                    // 内政実行
                    let kuni_mut = &mut kunis[i];
                    let (decision, _reasoning) =
                        CpuActionDecisionService::decide(daimyo.personality(), kuni_mut, turn, rng);

                    KuniActionService::apply_cpu_decision(kuni_mut, decision)?;
                    acted[i] = true;
                }
            }

            // 3. ターン終了時の季節イベント
            for kuni in kunis.iter_mut() {
                seasonal_service.process_end_turn_events(turn, kuni);
            }

            snapshots.push(SimulationSnapshot {
                turn,
                kuni_states: kunis.clone(),
                logs: turn_logs,
            });
        }

        Ok(snapshots)
    }
}

// --- Simulation用のインメモリリポジトリ ---

struct SimulationKuniRepo<'a> {
    kunis: &'a [Kuni],
}

#[async_trait]
impl<'a> KuniRepository for SimulationKuniRepo<'a> {
    async fn find_by_id(&self, id: &KuniId) -> Result<Option<Kuni>, DomainError> {
        Ok(self.kunis.iter().find(|k| k.id == *id).cloned())
    }
    async fn find_by_daimyo_id(&self, daimyo_id: &DaimyoId) -> Result<Vec<Kuni>, DomainError> {
        Ok(self
            .kunis
            .iter()
            .filter(|k| k.daimyo_id == *daimyo_id)
            .cloned()
            .collect())
    }
    async fn save(&self, _kuni: &Kuni) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_all(&self) -> Result<Vec<Kuni>, DomainError> {
        Ok(self.kunis.to_vec())
    }
    async fn clear(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

struct SimulationNeighborRepo<'a> {
    neighbors: &'a HashMap<KuniId, Vec<KuniId>>,
}

impl<'a> NeighborRepository for SimulationNeighborRepo<'a> {
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.neighbors.get(kuni_id).cloned().unwrap_or_default()
    }
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.neighbors.get(a).is_some_and(|l| l.contains(b))
    }
    fn reset(&self, _adjacency_map: HashMap<KuniId, Vec<KuniId>>) -> Result<(), DomainError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::daimyo_personality::DaimyoPersonality;
    use crate::domain::model::resource::{DevelopmentStats, Resource};
    use crate::domain::model::value_objects::IninFlag;
    use rand::SeedableRng;

    #[tokio::test]
    async fn test_run_simulation_log_includes_daimyo_name() {
        // 決定論的なテストのためにシード固定
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let daimyo_id1 = DaimyoId(1);
        // 軍事バイアスを最大にして出兵しやすくする
        let personality1 = DaimyoPersonality::new(2.0, 1.0, 1.0, 0.0).unwrap();
        let daimyo1 = Daimyo::new(daimyo_id1, "織田信長", personality1);

        let daimyo_id2 = DaimyoId(2);
        let daimyo2 = Daimyo::new(daimyo_id2, "今川義元", DaimyoPersonality::default());

        let daimyos = vec![daimyo1, daimyo2];

        let kuni1 = Kuni::new(
            KuniId(1),
            "尾張",
            daimyo_id1,
            Resource::new(100000, 100000, 100000, 100000),
            DevelopmentStats::new(10000, 10000, 60),
            IninFlag(false),
        );

        let kuni2 = Kuni::new(
            KuniId(2),
            "駿河",
            daimyo_id2,
            Resource::new(1000, 1000, 1000, 1000), // 非常に弱い
            DevelopmentStats::new(1000, 1000, 60),
            IninFlag(false),
        );

        let kunis = vec![kuni1, kuni2];

        let mut neighbors = HashMap::new();
        neighbors.insert(KuniId(1), vec![KuniId(2)]);
        neighbors.insert(KuniId(2), vec![KuniId(1)]);

        let snapshots =
            SimulationService::run_simulation(&daimyos, &kunis, &neighbors, 5, &mut rng)
                .await
                .unwrap();

        // ログに大名名が含まれているか確認
        let mut found_war_log = false;
        for snapshot in snapshots {
            for log in snapshot.logs {
                if log.contains("【戦争】") {
                    found_war_log = true;
                    assert!(
                        log.contains("織田信長") && log.contains("今川義元"),
                        "戦争ログに大名名が含まれていません: {}",
                        log
                    );
                }
            }
        }

        assert!(found_war_log, "テスト中に一度も戦争が発生しませんでした。");
    }
}
