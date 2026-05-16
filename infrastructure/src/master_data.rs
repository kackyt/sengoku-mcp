use engine::domain::error::DomainError;
use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::daimyo_personality::DaimyoPersonality;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::resource::{DevelopmentStats, Resource};
use engine::domain::model::value_objects::{DaimyoId, DisplayAmount, IninFlag, KuniId, Rate};
use engine::domain::repository::master_data_repository::{MasterDataBundle, MasterDataRepository};
use serde::Deserialize;
use std::collections::HashMap;

use crate::master_data_error::MasterDataError;

/// 大名情報のCSVレコード
#[derive(Debug, Deserialize)]
struct DaimyoRecord {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "名前")]
    name: String,
    #[serde(rename = "農業")]
    agriculture: f64,
    #[serde(rename = "商業")]
    commerce: f64,
    #[serde(rename = "軍事")]
    military: f64,
    #[serde(rename = "揺らぎ")]
    randomness: f64,
}

/// 隣接情報のCSVレコード
#[derive(Debug, Deserialize)]
struct NeighborRecord {
    #[serde(rename = "ID1")]
    id1: u32,
    #[serde(rename = "ID2")]
    id2: u32,
}

/// 国情報のCSVレコード
#[derive(Debug, Deserialize)]
struct KuniRecord {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "名前")]
    name: String,
    #[serde(rename = "大名ID")]
    daimyo_id: u32,
    #[serde(rename = "金")]
    kin: u32,
    #[serde(rename = "兵")]
    hei: u32,
    #[serde(rename = "米")]
    kome: u32,
    #[serde(rename = "人口")]
    jinko: u32,
    #[serde(rename = "石高")]
    kokudaka: u32,
    #[serde(rename = "町")]
    machi: u32,
    #[serde(rename = "忠誠")]
    tyu: u32,
}

/// CSVファイルからマスターデータを読み込むローダー
pub struct MasterDataLoader;

impl MasterDataRepository for MasterDataLoader {
    fn load(&self) -> Result<MasterDataBundle, DomainError> {
        Self::load().map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

impl MasterDataLoader {
    pub fn load() -> Result<MasterDataBundle, MasterDataError> {
        let daimyos = Self::load_daimyo()?;
        let mut daimyo_map = HashMap::new();
        for d in &daimyos {
            daimyo_map.insert(d.id, d.clone());
        }

        let kuni_csv = include_str!("../../static/master_data/kuni.csv");
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(kuni_csv.as_bytes());

        let mut kunis = Vec::new();
        let mut id_map = HashMap::<u32, KuniId>::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: KuniRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "不明".to_string(),
                reason: e.to_string(),
            })?;

            let daimyo_id = DaimyoId::new(record.daimyo_id);
            if !daimyo_map.contains_key(&daimyo_id) {
                return Err(MasterDataError::InvalidReference {
                    id: record.daimyo_id,
                });
            }

            let kuni_id = KuniId::new(record.id);
            id_map.insert(record.id, kuni_id);

            let resource = Resource {
                kin: DisplayAmount::new(record.kin).to_internal(),
                hei: DisplayAmount::new(record.hei).to_internal(),
                kome: DisplayAmount::new(record.kome).to_internal(),
                jinko: DisplayAmount::new(record.jinko).to_internal(),
            };

            let stats = DevelopmentStats {
                kokudaka: DisplayAmount::new(record.kokudaka).to_internal(),
                machi: DisplayAmount::new(record.machi).to_internal(),
                tyu: Rate::new(record.tyu),
            };

            let kuni = Kuni::new(
                kuni_id,
                record.name,
                daimyo_id,
                resource,
                stats,
                IninFlag(false),
            );
            kunis.push(kuni);
        }

        let adjacency_map = Self::load_neighbor(&id_map)?;

        Ok(MasterDataBundle {
            daimyos,
            kunis,
            adjacency_map,
        })
    }

    fn load_daimyo() -> Result<Vec<Daimyo>, MasterDataError> {
        let daimyo_csv = include_str!("../../static/master_data/daimyo.csv");
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(daimyo_csv.as_bytes());

        let mut daimyos = Vec::new();
        for (i, result) in rdr.deserialize().enumerate() {
            let record: DaimyoRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "不明".to_string(),
                reason: e.to_string(),
            })?;

            let personality = DaimyoPersonality::new(
                record.agriculture,
                record.commerce,
                record.military,
                record.randomness,
            )
            .map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "性格".to_string(),
                reason: e.to_string(),
            })?;

            daimyos.push(Daimyo::new(
                DaimyoId::new(record.id),
                record.name,
                personality,
            ));
        }
        Ok(daimyos)
    }

    fn load_neighbor(
        id_map: &HashMap<u32, KuniId>,
    ) -> Result<HashMap<KuniId, Vec<KuniId>>, MasterDataError> {
        let neighbor_csv = include_str!("../../static/master_data/neighbor.csv");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(neighbor_csv.as_bytes());

        let mut adjacency_map: HashMap<KuniId, Vec<KuniId>> = HashMap::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: NeighborRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "不明".to_string(),
                reason: e.to_string(),
            })?;

            let id1 = id_map
                .get(&record.id1)
                .ok_or(MasterDataError::InvalidReference { id: record.id1 })?;
            let id2 = id_map
                .get(&record.id2)
                .ok_or(MasterDataError::InvalidReference { id: record.id2 })?;
            adjacency_map.entry(*id1).or_default().push(*id2);
            adjacency_map.entry(*id2).or_default().push(*id1);
        }

        Ok(adjacency_map)
    }
}
