use crate::domain::model::value_objects::DisplayAmount;
use serde::{Deserialize, Serialize};

/// 他国の情報をまとめたDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtherCountriesInfoDTO {
    pub countries: Vec<CountryInfoDTO>,
}

/// 各国の統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryInfoDTO {
    pub kuni_id: u32,
    pub kuni_name: String,
    pub daimyo_name: String,
    pub kome: DisplayAmount,
    pub kin: DisplayAmount,
    pub hei: DisplayAmount,
    pub kokudaka: DisplayAmount,
    pub towns: DisplayAmount,
    pub tyu: u32,
}
