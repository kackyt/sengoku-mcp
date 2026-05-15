use crate::domain::error::DomainError;
use crate::domain::model::battle::Tactic;

pub struct TacticValidationService;

impl TacticValidationService {
    /// 戦術IDをTactic enumに変換します。攻撃側かどうかに応じて許可される戦術が異なります。
    pub fn parse_tactic(tactic_id: u32, is_attacker: bool) -> Result<Tactic, DomainError> {
        match tactic_id {
            1 => Ok(Tactic::Normal),
            2 => Ok(Tactic::Surprise),
            3 => Ok(Tactic::Fire),
            4 => Ok(Tactic::Inspire),
            5 if is_attacker => Ok(Tactic::Retreat),
            _ => Err(DomainError::InvalidTactic {
                tactic_id,
                is_attacker,
            }),
        }
    }
}
