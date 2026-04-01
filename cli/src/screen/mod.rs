use engine::domain::model::value_objects::{DaimyoId, KuniId};

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenState {
    Title,
    SelectDaimyo {
        cursor: usize,
    },
    Domestic {
        selected_kuni: KuniId,
        cursor: usize,
        sub_state: DomesticSubState,
    },
    War {
        attacker_kuni: KuniId,
        defender_kuni: KuniId,
        cursor: usize,
        sub_state: WarSubState,
    },
    GameOver {
        winner: DaimyoId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomesticSubState {
    Normal,
    InputAmount {
        command: DomesticCommand,
        input: String,
    },
    SelectTargetKuni {
        command: DomesticCommand,
        cursor: usize,
    },
    ShowMessage {
        message: String,
        next_state: Box<DomesticSubState>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarSubState {
    Normal,
    SelectTactic,
    ShowMessage {
        message: String,
        next_state: Box<WarSubState>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DomesticCommand {
    War,
    SellRice,
    BuyRice,
    Develop,
    BuildTown,
    Hire,
    Dismiss,
    Give,
    Transport,
    Delegate,
    Undelegate,
    Information,
    Exit,
}
