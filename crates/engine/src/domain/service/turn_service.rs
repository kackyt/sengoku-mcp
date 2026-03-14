use crate::domain::model::kuni::Kuni;
use rand::Rng;

pub struct TurnService;

impl TurnService {
    pub fn process_season(turn: u32, mut kunis: Vec<Kuni>) -> Vec<Kuni> {
        let mut rng = rand::thread_rng();

        for kuni in &mut kunis {
            // Disaster check (1/40 chance)
            if rng.gen_range(0..40) == 0 {
                if turn % 4 == 1 {
                    // Plague
                    let drop: u32 = rng.gen_range(5..=9);
                    let jinko_loss = kuni.resource.jinko.value() * drop / 100;
                    kuni.modify_jinko(-(jinko_loss as i32));
                    // Additional stat losses
                    kuni.modify_tyu(-20);
                } else {
                    // Famine
                    let drop: u32 = rng.gen_range(5..=19);
                    let jinko_loss = kuni.resource.jinko.value() * drop / 100;
                    kuni.modify_jinko(-(jinko_loss as i32));
                    // Additional stat losses
                    kuni.modify_tyu(-15);
                }
            }

            // Population growth (turn % 4 == 0)
            if turn.is_multiple_of(4) {
                let growth: u32 = rng.gen_range(10..=12);
                let jinko_gain = kuni.resource.jinko.value() * growth / 100;
                kuni.modify_jinko(jinko_gain as i32);
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

                kuni.add_resource(kin_gain, 0, kome_gain);
            }
        }

        kunis
    }
}
