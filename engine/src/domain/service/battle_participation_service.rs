use crate::domain::model::battle::WarStatus;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::DaimyoId;

pub struct BattleParticipationService;

impl BattleParticipationService {
    /// プレイヤーが大名として戦争（攻撃側または防御側）に参加しているか判定します
    pub fn is_player_participating(
        player_id: Option<DaimyoId>,
        attacker_kuni: Option<&Kuni>,
        defender_kuni: Option<&Kuni>,
    ) -> bool {
        let Some(pid) = player_id else {
            return false;
        };

        let is_attacker = attacker_kuni.is_some_and(|k| k.daimyo_id == pid);
        let is_defender = defender_kuni.is_some_and(|k| k.daimyo_id == pid);

        is_attacker || is_defender
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
