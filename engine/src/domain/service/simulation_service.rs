use crate::domain::model::{daimyo::Daimyo, kuni::Kuni, value_objects::TurnNumber};
use crate::domain::service::{
    cpu_action_decision_service::CpuActionDecisionService, kuni_action_service::KuniActionService,
    seasonal_event_service::SeasonalEventService,
};
use rand::Rng;
use std::collections::HashMap;

/// シミュレーションの結果のスナップショット
#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub turn: TurnNumber,
    pub kuni_states: Vec<Kuni>,
}

pub struct SimulationService;

impl SimulationService {
    /// 指定された大名と国のリストを用いて、指定ターン数分シミュレーションを実行します。
    pub fn run_simulation(
        daimyos: &[Daimyo],
        initial_kunis: &[Kuni],
        num_turns: u32,
        rng: &mut impl Rng,
    ) -> Vec<SimulationSnapshot> {
        let mut kunis = initial_kunis.to_vec();
        let mut snapshots = Vec::new();
        let seasonal_service = SeasonalEventService::new();

        let daimyo_map: HashMap<_, _> = daimyos.iter().map(|d| (d.id, d)).collect();

        for t in 1..=num_turns {
            let turn = TurnNumber::new(t);

            // 1. ターン開始時の季節イベント
            for kuni in kunis.iter_mut() {
                seasonal_service.process_start_turn_events(turn, kuni);
            }

            // 2. 各大名の行動
            for kuni in kunis.iter_mut() {
                let daimyo = daimyo_map.get(&kuni.daimyo_id).expect("Daimyo not found");
                let (decision, _reasoning) =
                    CpuActionDecisionService::decide(&daimyo.personality, kuni, turn, rng);

                let _ = KuniActionService::apply_cpu_decision(kuni, decision);
            }

            // 3. ターン終了時の季節イベント
            for kuni in kunis.iter_mut() {
                seasonal_service.process_end_turn_events(turn, kuni);
            }

            snapshots.push(SimulationSnapshot {
                turn,
                kuni_states: kunis.clone(),
            });
        }

        snapshots
    }
}
