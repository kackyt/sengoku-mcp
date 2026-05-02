use crate::domain::model::event::SeasonalEventEffect;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{KuniId, TurnNumber};
use crate::domain::service::seasonal_event_service::SeasonalEventService;
use rand::seq::SliceRandom;
use rand::Rng;

pub struct TurnService;

impl TurnService {
    /// ターンの行動順序（国のIDの配列）をランダムに決定する
    pub fn determine_action_order(kunis: &[Kuni], rng: &mut impl Rng) -> Vec<KuniId> {
        let mut order: Vec<KuniId> = kunis.iter().map(|k| k.id).collect();
        order.shuffle(rng);
        order
    }

    /// ターン開始時の季節イベント（洪水・疫病・反乱）を処理し、発生したイベント効果を返す
    pub fn process_start_turn_events(
        turn: TurnNumber,
        kunis: &mut [Kuni],
    ) -> Vec<SeasonalEventEffect> {
        let service = SeasonalEventService::new();
        let mut all_effects = Vec::new();
        for kuni in kunis.iter_mut() {
            let effects = service.process_start_turn_events(turn, kuni);
            all_effects.extend(effects);
        }
        all_effects
    }

    /// ターン終了時の季節イベント（人口増加・資源生成）を処理し、発生したイベント効果を返す
    pub fn process_end_turn_events(
        turn: TurnNumber,
        kunis: &mut [Kuni],
    ) -> Vec<SeasonalEventEffect> {
        let service = SeasonalEventService::new();
        let mut all_effects = Vec::new();
        for kuni in kunis.iter_mut() {
            let effects = service.process_end_turn_events(turn, kuni);
            all_effects.extend(effects);
        }
        all_effects
    }
}
