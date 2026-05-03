use crate::domain::{
    error::DomainError,
    model::action_log::{ActionLogCategory, ActionLogEntry, ActionLogVisibility},
    model::value_objects::{Amount, DisplayAmount, IninFlag, KuniId},
    repository::action_log_repository::ActionLogRepository,
    repository::game_state_repository::GameStateRepository,
    repository::kuni_repository::KuniRepository,
    repository::neighbor_repository::NeighborRepository,
};
use std::sync::Arc;

/// 内政に関するユースケース
#[allow(dead_code)]
pub struct DomesticUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
    action_log_repo: Arc<dyn ActionLogRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
}

impl DomesticUseCase {
    /// 新しい内政ユースケースを作成します
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        action_log_repo: Arc<dyn ActionLogRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            neighbor_repo,
            action_log_repo,
            game_state_repo,
        }
    }

    /// 米を売却します
    pub async fn sell_rice(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<DisplayAmount, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.sell_rice(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!("{}：米を売却し、金{}を得ました", kuni.name.0, gain.value());
        let detail = format!(
            "売却量: {}, 残金: {}, 残米: {}",
            amount.value(),
            kuni.resource.kin.value(),
            kuni.resource.kome.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(gain)
    }

    /// 米を購入します
    pub async fn buy_rice(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<DisplayAmount, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let cost = kuni.buy_rice(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!(
            "{}：米を{}購入しました（金{}を消費）",
            kuni.name.0,
            amount.value(),
            cost.value()
        );
        let detail = format!(
            "購入量: {}, 残金: {}, 残米: {}",
            amount.value(),
            kuni.resource.kin.value(),
            kuni.resource.kome.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(cost)
    }

    /// 開墾を行います
    pub async fn develop_land(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<DisplayAmount, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.develop_land(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!(
            "{}：開墾し、石高が{}上昇しました",
            kuni.name.0,
            gain.value()
        );
        let detail = format!(
            "投資額: {}, 開墾後石高: {}",
            amount.value(),
            kuni.stats.kokudaka.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(gain)
    }

    /// 町作りを行います
    pub async fn build_town(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<DisplayAmount, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.build_town(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!(
            "{}：町を整備し、町が{}上昇しました",
            kuni.name.0,
            gain.value()
        );
        let detail = format!(
            "投資額: {}, 整備後町: {}",
            amount.value(),
            kuni.stats.machi.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(gain)
    }

    /// 兵を徴募します
    pub async fn recruit(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        kuni.recruit_troops(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!("{}：兵を{}徴募しました", kuni.name.0, amount.value());
        let detail = format!(
            "徴募後 兵力: {}, 人口: {}, 忠誠度: {}",
            kuni.resource.hei.value(),
            kuni.resource.jinko.value(),
            kuni.stats.tyu.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(())
    }

    /// 兵を解雇します
    pub async fn dismiss(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        kuni.dismiss_troops(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!("{}：兵を{}解雇しました", kuni.name.0, amount.value());
        let detail = format!(
            "解雇後 兵力: {}, 人口: {}, 忠誠度: {}",
            kuni.resource.hei.value(),
            kuni.resource.jinko.value(),
            kuni.stats.tyu.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(())
    }

    /// 施しを行います
    pub async fn give_charity(
        &self,
        kuni_id: KuniId,
        amount: DisplayAmount,
    ) -> Result<u32, anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;

        let gain = kuni.give_charity(amount)?;

        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!("{}：施しを行い、忠誠度が{}上昇しました", kuni.name.0, gain);
        let detail = format!(
            "消費米: {}, 施し後忠誠度: {}",
            amount.value(),
            kuni.stats.tyu.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(gain)
    }

    /// 輸送を行います
    pub async fn transport(
        &self,
        from_kuni_id: KuniId,
        to_kuni_id: KuniId,
        kin: DisplayAmount,
        hei: DisplayAmount,
        kome: DisplayAmount,
    ) -> Result<(), anyhow::Error> {
        let mut from_kuni = self
            .kuni_repo
            .find_by_id(&from_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("送り元の国が見つかりません: {:?}", from_kuni_id))?;
        let mut to_kuni = self
            .kuni_repo
            .find_by_id(&to_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("送り先の国が見つかりません: {:?}", to_kuni_id))?;

        if !self.neighbor_repo.are_adjacent(&from_kuni_id, &to_kuni_id) {
            return Err(DomainError::NotAdjacent.into());
        }

        let kin_internal = kin.to_internal();
        let hei_internal = hei.to_internal();
        let kome_internal = kome.to_internal();

        from_kuni.consume_resource(kin_internal, hei_internal, kome_internal, Amount::zero())?;
        to_kuni
            .resource
            .add(kin_internal, hei_internal, kome_internal, Amount::zero());

        self.kuni_repo.save(&from_kuni).await?;
        self.kuni_repo.save(&to_kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let message = format!(
            "{}→{}：資源を輸送しました（金:{} 兵:{} 米:{}）",
            from_kuni.name.0,
            to_kuni.name.0,
            kin.value(),
            hei.value(),
            kome.value()
        );
        let detail = format!(
            "輸送後 {} 金:{} 兵:{} 米:{}, {} 金:{} 兵:{} 米:{}",
            from_kuni.name.0,
            from_kuni.resource.kin.value(),
            from_kuni.resource.hei.value(),
            from_kuni.resource.kome.value(),
            to_kuni.name.0,
            to_kuni.resource.kin.value(),
            to_kuni.resource.hei.value(),
            to_kuni.resource.kome.value()
        );
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(())
    }

    /// 指定した割合で資源を輸送します（内政コマンド用）
    pub async fn transport_with_rate(
        &self,
        from_kuni_id: KuniId,
        to_kuni_id: KuniId,
        rate_percent: u32,
    ) -> Result<(), anyhow::Error> {
        let from_kuni = self
            .kuni_repo
            .find_by_id(&from_kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("送り元の国が見つかりません: {:?}", from_kuni_id))?;

        let kin = from_kuni.resource.kin.mul_percent(rate_percent);
        let hei = from_kuni.resource.hei.mul_percent(rate_percent);
        let kome = from_kuni.resource.kome.mul_percent(rate_percent);

        self.transport(
            from_kuni_id,
            to_kuni_id,
            kin.to_display(),
            hei.to_display(),
            kome.to_display(),
        )
        .await
    }

    /// 委任状態を設定します
    pub async fn set_delegation(
        &self,
        kuni_id: KuniId,
        delegate: bool,
    ) -> Result<(), anyhow::Error> {
        let mut kuni = self
            .kuni_repo
            .find_by_id(&kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", kuni_id))?;
        kuni.set_inin(IninFlag::new(delegate));
        self.kuni_repo.save(&kuni).await?;

        let turn = self
            .game_state_repo
            .get()
            .await?
            .map(|s| s.current_turn())
            .unwrap_or(crate::domain::model::value_objects::TurnNumber::new(1));
        let state_str = if delegate { "ON" } else { "OFF" };
        let message = format!("{}：委任を{}にしました", kuni.name.0, state_str);
        let detail = format!("切替後の状態: {}", state_str);
        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogCategory::Domestic,
            ActionLogVisibility::Player,
            turn,
            message,
            detail,
        ));

        Ok(())
    }
}
