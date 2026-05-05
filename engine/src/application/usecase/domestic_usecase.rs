use crate::domain::{
    error::DomainError,
    model::action_log::{ActionLogEntry, ActionLogEvent, ActionLogVisibility, DomesticLogEvent},
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::RiceSold {
                kuni_name: kuni.name.clone(),
                gain: gain.to_internal(),
                amount,
                rem_kin: kuni.resource.kin,
                rem_kome: kuni.resource.kome,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::RiceBought {
                kuni_name: kuni.name.clone(),
                cost: cost.to_internal(),
                amount,
                rem_kin: kuni.resource.kin,
                rem_kome: kuni.resource.kome,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::LandReclaimed {
                kuni_name: kuni.name.clone(),
                gain: gain.to_internal(),
                cost: amount.to_internal(),
                new_tyu: kuni.stats.tyu,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::TownDeveloped {
                kuni_name: kuni.name.clone(),
                gain: gain.to_internal(),
                cost: amount.to_internal(),
                new_tyu: kuni.stats.tyu,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::TroopsDrafted {
                kuni_name: kuni.name.clone(),
                amount,
                rem_hei: kuni.resource.hei,
                rem_jinko: kuni.resource.jinko,
                new_tyu: kuni.stats.tyu,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::TroopsDismissed {
                kuni_name: kuni.name.clone(),
                amount,
                rem_hei: kuni.resource.hei,
                rem_jinko: kuni.resource.jinko,
                new_tyu: kuni.stats.tyu,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::CharityPerformed {
                kuni_name: kuni.name.clone(),
                gain_tyu: crate::domain::model::value_objects::Rate::new(gain),
                cost: amount.to_internal(),
                rem_tyu: kuni.stats.tyu,
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::ResourcesTransported {
                from_kuni: from_kuni.name.clone(),
                to_kuni: to_kuni.name.clone(),
                kin: kin.to_internal(),
                hei: hei.to_internal(),
                kome: kome.to_internal(),
            }),
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

        let _ = self.action_log_repo.save(ActionLogEntry::new(
            ActionLogVisibility::Player,
            turn,
            ActionLogEvent::Domestic(DomesticLogEvent::DelegationChanged {
                kuni_name: kuni.name.clone(),
                enabled: delegate,
            }),
        ));

        Ok(())
    }
}
