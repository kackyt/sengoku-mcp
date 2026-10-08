use crate::domain::model::value_objects::{DaimyoId, KuniId};

/// 国ごとの支配大名（勢力図）を表すDTO
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerritoryDTO {
    /// 国ID
    pub kuni_id: KuniId,
    /// 国名
    pub kuni_name: String,
    /// 支配している大名のID
    pub daimyo_id: DaimyoId,
    /// 支配している大名名（大名が見つからない場合は「不明」）
    pub daimyo_name: String,
}
