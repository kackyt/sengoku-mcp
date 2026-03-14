use crate::domain::model::kuni::Kuni;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tactic {
    Normal,
    Surprise,
    Fire,
    Inspire,
}

#[derive(Debug)]
pub struct BattleResult {
    pub attacker_kuni: Kuni,
    pub defender_kuni: Kuni,
    pub winner: Option<BattleSide>, // None if battle still ongoing
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide {
    Attacker,
    Defender,
}

pub struct BattleService;

impl BattleService {
    pub fn calculate_turn(
        mut attacker: Kuni,
        mut defender: Kuni,
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
        attacker_troops: u32,
    ) -> Result<BattleResult, anyhow::Error> {
        // Implement tactics effects

        // Simplified damage calculation based on PRD
        let mut base_damage = attacker_troops;

        match (attacker_tactic, defender_tactic) {
            (Tactic::Normal, Tactic::Normal) => {
                base_damage = (base_damage * 180) / 100;
            }
            (Tactic::Surprise, Tactic::Normal) => {
                base_damage = (base_damage * 40) / 100;
                defender.modify_tyu(-10); // Morale drop (simplified using tyu as morale for now)
                attacker.modify_tyu(10);
            }
            (Tactic::Surprise, Tactic::Surprise) => {
                base_damage = (base_damage * 300) / 100;
                attacker.modify_tyu(-10);
            }
            (Tactic::Fire, Tactic::Fire) => {
                // Attacker loses troops
                let loss = (attacker.resource.hei.value() * 30) / 100;
                let _ = attacker.resource.consume(0, loss, 0);
                attacker.modify_tyu(-10);
            }
            (Tactic::Fire, _) => {
                // Defender loses food
                let loss = (defender.resource.kome.value() * 50) / 100;
                let _ = defender.resource.consume(0, 0, loss);
                defender.modify_tyu(-10);
                attacker.modify_tyu(10);
            }
            (_, Tactic::Inspire) => {
                defender.modify_tyu(15);
            }
            _ => {
                base_damage = (base_damage * 60) / 100;
            }
        }

        // Apply damage
        let _ = defender.resource.consume(0, base_damage, 0);

        // 30% food consumption for troops
        let food_cost = (attacker_troops * 30) / 100;
        if attacker.resource.consume(0, 0, food_cost).is_err() {
            attacker.modify_tyu(-40); // Large morale drop on starvation
        }

        // Check victory conditions
        let winner = if defender.resource.hei.value() == 0
            || defender.resource.kome.value() == 0
            || defender.stats.tyu.value() == 0
        {
            Some(BattleSide::Attacker)
        } else if attacker.resource.hei.value() == 0
            || attacker.resource.kome.value() == 0
            || attacker.stats.tyu.value() == 0
        {
            Some(BattleSide::Defender)
        } else {
            None
        };

        // If attacker wins, they take the territory and remaining resources
        if winner == Some(BattleSide::Attacker) {
            attacker.add_resource(
                0,
                defender.resource.hei.value(),
                defender.resource.kome.value(),
            );
            // In a real implementation, ownership of the kuni would transfer here via UseCase
        }

        Ok(BattleResult {
            attacker_kuni: attacker,
            defender_kuni: defender,
            winner,
        })
    }
}
