use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaimyoSummaryDto {
    pub id: u32,
    pub name: String,
}
