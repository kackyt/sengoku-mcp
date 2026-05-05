use crate::domain::error::DomainError;
use crate::domain::model::kuni::Kuni;
use crate::domain::service::cpu_action_decision_service::CpuActionDecision;

/// 国に対するアクションを実行するためのドメインサービス
pub struct KuniActionService;

impl KuniActionService {
    /// CPUの決定事項を国に適用します
    pub fn apply_cpu_decision(
        kuni: &mut Kuni,
        decision: CpuActionDecision,
    ) -> Result<String, DomainError> {
        match decision {
            CpuActionDecision::DevelopLand {
                target_kuni_id,
                amount,
            } => {
                if target_kuni_id != kuni.id {
                    return Err(DomainError::InvalidOperation(
                        "対象国IDが一致しません".to_string(),
                    ));
                }
                kuni.develop_land(amount)?;
                Ok("開墾を行いました".to_string())
            }
            CpuActionDecision::BuildTown {
                target_kuni_id,
                amount,
            } => {
                if target_kuni_id != kuni.id {
                    return Err(DomainError::InvalidOperation(
                        "対象国IDが一致しません".to_string(),
                    ));
                }
                kuni.build_town(amount)?;
                Ok("町造りを行いました".to_string())
            }
            CpuActionDecision::SellRice { amount, .. } => {
                kuni.sell_rice(amount)?;
                Ok("米売りを行いました".to_string())
            }
            CpuActionDecision::BuyRice { amount, .. } => {
                kuni.buy_rice(amount)?;
                Ok("米買いを行いました".to_string())
            }
            CpuActionDecision::Recruit { amount, .. } => {
                kuni.recruit_troops(amount)?;
                Ok("雇用を行いました".to_string())
            }
            CpuActionDecision::Dismiss { amount, .. } => {
                kuni.dismiss_troops(amount)?;
                Ok("解雇を行いました".to_string())
            }
            CpuActionDecision::GiveCharity { amount, .. } => {
                kuni.give_charity(amount)?;
                Ok("施しを行いました".to_string())
            }
            CpuActionDecision::Rest => Ok("休息しました".to_string()),
            _ => Err(DomainError::InvalidOperation(
                "非対応または非内政のアクションです".to_string(),
            )),
        }
    }
}
