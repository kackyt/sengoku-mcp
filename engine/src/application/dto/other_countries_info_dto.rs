use serde::{Deserialize, Serialize};
use crate::domain::model::value_objects::DisplayAmount;

/// 他国の情報をまとめたDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtherCountriesInfoDTO {
    pub countries: Vec<CountryInfoDTO>,
}

/// 各大名の統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryInfoDTO {
    pub daimyo_id: u32,
    pub daimyo_name: String,
    pub kome: DisplayAmount,
    pub kin: DisplayAmount,
    pub hei: DisplayAmount,
    pub kokudaka: DisplayAmount,
    pub towns: DisplayAmount,
    /// 領地の平均忠誠度
    pub tyu_avg: u32,
}
