use crate::domain::model::battle::Tactic;
use crate::domain::model::event::SeasonalEventType;
use crate::domain::model::value_objects::{
    Amount, DaimyoId, DisplayAmount, KuniName, Rate, TurnNumber,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionLogCategory {
    Domestic,
    War,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLogVisibility {
    Public,
    Player,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomesticLogEvent {
    RiceSold {
        kuni_name: KuniName,
        gain: Amount,
        amount: DisplayAmount,
        rem_kin: Amount,
        rem_kome: Amount,
    },
    RiceBought {
        kuni_name: KuniName,
        cost: Amount,
        amount: DisplayAmount,
        rem_kin: Amount,
        rem_kome: Amount,
    },
    LandReclaimed {
        kuni_name: KuniName,
        gain: Amount,
        cost: Amount,
        new_tyu: Rate,
    },
    TownDeveloped {
        kuni_name: KuniName,
        gain: Amount,
        cost: Amount,
        new_tyu: Rate,
    },
    TroopsDrafted {
        kuni_name: KuniName,
        amount: DisplayAmount,
        rem_hei: Amount,
        rem_jinko: Amount,
        new_tyu: Rate,
    },
    TroopsDismissed {
        kuni_name: KuniName,
        amount: DisplayAmount,
        rem_hei: Amount,
        rem_jinko: Amount,
        new_tyu: Rate,
    },
    CharityPerformed {
        kuni_name: KuniName,
        gain_tyu: Rate,
        cost: Amount,
        rem_tyu: Rate,
    },
    ResourcesTransported {
        from_kuni: KuniName,
        to_kuni: KuniName,
        kin: Amount,
        hei: Amount,
        kome: Amount,
    },
    DelegationChanged {
        kuni_name: KuniName,
        enabled: bool,
    },
    CpuAction {
        daimyo_id: DaimyoId,
        action_msg: String,
        reasoning: Option<String>,
    },
    TurnStart {
        turn: TurnNumber,
        season: u32,
    },
    SeasonalEvent {
        event_type: SeasonalEventType,
        kuni_names: Vec<KuniName>,
    },
    WarStarted {
        attacker_name: KuniName,
        defender_name: KuniName,
    },
    WarAttackerOccupied {
        home_name: KuniName,
        occupied_name: KuniName,
    },
    WarDefenderDefended {
        defender_name: KuniName,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarLogEvent {
    CpuDefenderTactic {
        tactic: Tactic,
    },
    Damage {
        attacker_tactic: Tactic,
        defender_tactic: Tactic,
        attacker_damage: Amount,
        defender_damage: Amount,
    },
    AttackerVictory {
        home_name: KuniName,
        attacker_id: DaimyoId,
        occupied_name: KuniName,
        defender_id: DaimyoId,
    },
    DefenderVictory {
        home_name: KuniName,
        attacker_id: DaimyoId,
        defender_id: DaimyoId,
    },
    WarStarted {
        attacker_name: KuniName,
        defender_name: KuniName,
        attacker_id: DaimyoId,
        defender_id: DaimyoId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionLogEvent {
    Domestic(DomesticLogEvent),
    War(WarLogEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionLogEntry {
    pub visibility: ActionLogVisibility,
    pub turn: TurnNumber,
    pub event: ActionLogEvent,
}

impl ActionLogEntry {
    pub fn new(visibility: ActionLogVisibility, turn: TurnNumber, event: ActionLogEvent) -> Self {
        Self {
            visibility,
            turn,
            event,
        }
    }

    pub fn category(&self) -> ActionLogCategory {
        match self.event {
            ActionLogEvent::Domestic(_) => ActionLogCategory::Domestic,
            ActionLogEvent::War(_) => ActionLogCategory::War,
        }
    }
}
