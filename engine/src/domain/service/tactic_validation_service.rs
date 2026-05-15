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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::battle::Tactic;

    #[test]
    fn test_parse_tactic_attacker() {
        assert_eq!(
            TacticValidationService::parse_tactic(1, true).unwrap(),
            Tactic::Normal
        );
        assert_eq!(
            TacticValidationService::parse_tactic(2, true).unwrap(),
            Tactic::Surprise
        );
        assert_eq!(
            TacticValidationService::parse_tactic(3, true).unwrap(),
            Tactic::Fire
        );
        assert_eq!(
            TacticValidationService::parse_tactic(4, true).unwrap(),
            Tactic::Inspire
        );
        assert_eq!(
            TacticValidationService::parse_tactic(5, true).unwrap(),
            Tactic::Retreat
        );
        assert!(TacticValidationService::parse_tactic(6, true).is_err());
    }

    #[test]
    fn test_parse_tactic_defender() {
        assert_eq!(
            TacticValidationService::parse_tactic(1, false).unwrap(),
            Tactic::Normal
        );
        assert_eq!(
            TacticValidationService::parse_tactic(2, false).unwrap(),
            Tactic::Surprise
        );
        assert_eq!(
            TacticValidationService::parse_tactic(3, false).unwrap(),
            Tactic::Fire
        );
        assert_eq!(
            TacticValidationService::parse_tactic(4, false).unwrap(),
            Tactic::Inspire
        );
        // Defender cannot retreat
        assert!(TacticValidationService::parse_tactic(5, false).is_err());
    }
}
