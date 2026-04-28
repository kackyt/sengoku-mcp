use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{Amount, KuniId, Rate};
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

    #[allow(clippy::manual_is_multiple_of)]
    pub fn process_season(turn: u32, mut kunis: Vec<Kuni>, rng: &mut impl Rng) -> Vec<Kuni> {
        for kuni in &mut kunis {
            // Disaster check (1/40 chance)
            if rng.gen_range(0..40) == 0 {
                if turn % 4 == 1 {
                    // Plague
                    let drop: u32 = rng.gen_range(5..=9);
                    let jinko_loss = kuni.resource.jinko.value() * drop / 100;
                    kuni.resource.jinko -= Amount::new(jinko_loss);
                    // Additional stat losses
                    kuni.stats.tyu -= Rate::new(20);
                } else {
                    // Famine
                    let drop: u32 = rng.gen_range(5..=19);
                    let jinko_loss = kuni.resource.jinko.value() * drop / 100;
                    kuni.resource.jinko -= Amount::new(jinko_loss);
                    // Additional stat losses
                    kuni.stats.tyu -= Rate::new(15);
                }
            }

            // Population growth (turn % 4 == 0)
            if turn % 4 == 0 {
                let growth: u32 = rng.gen_range(10..=12);
                let jinko_gain = kuni.resource.jinko.value() * growth / 100;
                kuni.resource.jinko += Amount::new(jinko_gain);
            }

            // Resource generation (turn % 4 == 2)
            if turn % 4 == 2 {
                let tyu = kuni.stats.tyu.value();
                let jinko = kuni.resource.jinko.value();
                let machi = kuni.stats.machi.value();
                let kokudaka = kuni.stats.kokudaka.value();

                let kin_gain = tyu * rng.gen_range(3..=4) / 100
                    + jinko * rng.gen_range(10..=14) / 100
                    + machi * rng.gen_range(25..=39) / 100;

                let kome_gain = tyu * rng.gen_range(3..=4) / 100
                    + jinko * rng.gen_range(10..=14) / 100
                    + kokudaka * rng.gen_range(25..=39) / 100;

                kuni.resource.add(
                    Amount::new(kin_gain),
                    Amount::new(0),
                    Amount::new(kome_gain),
                    Amount::new(0),
                );
            }
        }

        kunis
    }
}
