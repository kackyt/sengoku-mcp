use crate::domain::model::battle::WarStatus;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::DaimyoId;

pub struct BattleParticipationService;

impl BattleParticipationService {
    /// プレイヤーが攻撃側として参加しているか判定します
    pub fn is_player_attacker(
        status: &WarStatus,
        player_daimyo_id: &DaimyoId,
        kunis: &[Kuni],
    ) -> bool {
        let my_kuni_ids: std::collections::HashSet<_> = kunis
            .iter()
            .filter(|k| k.daimyo_id == *player_daimyo_id)
            .map(|k| k.id)
            .collect();
        my_kuni_ids.contains(&status.attacker.kuni_id)
    }

    /// プレイヤーが防御側として参加しているか判定します
    pub fn is_player_defender(
        status: &WarStatus,
        player_daimyo_id: &DaimyoId,
        kunis: &[Kuni],
    ) -> bool {
        let my_kuni_ids: std::collections::HashSet<_> = kunis
            .iter()
            .filter(|k| k.daimyo_id == *player_daimyo_id)
            .map(|k| k.id)
            .collect();
        my_kuni_ids.contains(&status.defender.kuni_id)
    }

    /// プレイヤーが大名として戦争（攻撃側または防御側）に参加しているか判定します
    pub fn is_player_participating(
        status: &WarStatus,
        player_daimyo_id: &DaimyoId,
        kunis: &[Kuni],
    ) -> bool {
        Self::is_player_attacker(status, player_daimyo_id, kunis)
            || Self::is_player_defender(status, player_daimyo_id, kunis)
    }

    /// プレイヤーが防御側となる合戦を検索します
    pub fn find_defense_battle_for_player<'a>(
        player_id: DaimyoId,
        active_battles: &'a [WarStatus],
        all_kunis: &[Kuni],
    ) -> Option<&'a WarStatus> {
        active_battles.iter().find(|b| {
            all_kunis
                .iter()
                .any(|k| k.id == b.defender.kuni_id && k.daimyo_id == player_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::battle::{ArmyStatus, BattleAdvantage};
    use crate::domain::model::kuni::Kuni;
    use crate::domain::model::resource::{DevelopmentStats, Resource};
    use crate::domain::model::value_objects::DaimyoId;
    use crate::domain::model::value_objects::{IninFlag, KuniId, Rate};

    fn create_mock_kuni(id: u32, daimyo_id: u32) -> Kuni {
        Kuni::new(
            KuniId::new(id),
            "Test".to_string(),
            DaimyoId::new(daimyo_id),
            Resource::new(0, 0, 0, 0),
            DevelopmentStats::new(0, 0, 0),
            IninFlag::new(false),
        )
    }

    fn create_mock_war_status(attacker_kuni_id: u32, defender_kuni_id: u32) -> WarStatus {
        WarStatus {
            attacker: ArmyStatus {
                kuni_id: KuniId::new(attacker_kuni_id),
                hei: crate::domain::model::value_objects::Amount::zero(),
                kome: crate::domain::model::value_objects::Amount::zero(),
                morale: Rate::new(0),
            },
            defender: ArmyStatus {
                kuni_id: KuniId::new(defender_kuni_id),
                hei: crate::domain::model::value_objects::Amount::zero(),
                kome: crate::domain::model::value_objects::Amount::zero(),
                morale: Rate::new(0),
            },
            winner: None,
            advantage: BattleAdvantage::Even,
        }
    }

    #[test]
    fn test_participation_logic() {
        let player_id = DaimyoId::new(1);
        let kunis = vec![create_mock_kuni(10, 1), create_mock_kuni(20, 2)];

        // Player is attacker
        let status = create_mock_war_status(10, 20);
        assert!(BattleParticipationService::is_player_participating(
            &status, &player_id, &kunis
        ));
        assert!(BattleParticipationService::is_player_attacker(
            &status, &player_id, &kunis
        ));
        assert!(!BattleParticipationService::is_player_defender(
            &status, &player_id, &kunis
        ));

        // Player is defender
        let status = create_mock_war_status(20, 10);
        assert!(BattleParticipationService::is_player_participating(
            &status, &player_id, &kunis
        ));
        assert!(!BattleParticipationService::is_player_attacker(
            &status, &player_id, &kunis
        ));
        assert!(BattleParticipationService::is_player_defender(
            &status, &player_id, &kunis
        ));

        // Player is not involved
        let status = create_mock_war_status(20, 30);
        assert!(!BattleParticipationService::is_player_participating(
            &status, &player_id, &kunis
        ));
    }

    #[test]
    fn test_find_defense_battle() {
        let player_id = DaimyoId::new(1);
        let kunis = vec![create_mock_kuni(10, 1), create_mock_kuni(20, 2)];
        let battles = vec![
            create_mock_war_status(20, 10), // Player is defender
            create_mock_war_status(10, 20), // Player is attacker
        ];

        let defense_battle =
            BattleParticipationService::find_defense_battle_for_player(player_id, &battles, &kunis);
        assert!(defense_battle.is_some());
        assert_eq!(defense_battle.unwrap().defender.kuni_id, KuniId::new(10));
    }
}
