use crate::domain::error::DomainError;
use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::KuniId;
use std::collections::HashMap;

/// マスターデータのロード結果をまとめた構造体
pub struct MasterDataBundle {
    /// ロードされた大名のリスト
    pub daimyos: Vec<Daimyo>,
    /// ロードされた国のリスト
    pub kunis: Vec<Kuni>,
    /// ロードされた隣接情報 (内部ID間)
    pub adjacency_map: HashMap<KuniId, Vec<KuniId>>,
}

/// マスターデータを取得するためのリポジトリのインターフェース
pub trait MasterDataRepository: Send + Sync {
    /// 全てのマスターデータをロードします
    fn load(&self) -> Result<MasterDataBundle, DomainError>;
}
