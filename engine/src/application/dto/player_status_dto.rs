use crate::domain::model::value_objects::{DisplayAmount, KuniId};

#[derive(Debug, Clone)]
pub struct PlayerStatusDTO {
    pub current_turn: u32,
    pub current_daimyo_name: String,
    pub kunis: Vec<KuniStatusDTO>,
    pub defense_alerts: Vec<DefenseAlertDTO>,
}

#[derive(Debug, Clone)]
pub struct KuniStatusDTO {
    pub id: KuniId,
    pub name: String,
    pub kin: DisplayAmount,
    pub kome: DisplayAmount,
    pub hei: DisplayAmount,
    pub kokudaka: DisplayAmount,
    pub machi: DisplayAmount,
    pub tyu: u32,
}

#[derive(Debug, Clone)]
pub struct DefenseAlertDTO {
    pub attacker_kuni_name: String,
    pub defender_kuni_name: String,
    pub enemy_hei: DisplayAmount,
}
