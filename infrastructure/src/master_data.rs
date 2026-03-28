use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::resource::{DevelopmentStats, Resource};
use engine::domain::model::value_objects::{Amount, DaimyoId, IninFlag, KuniId, Rate};
use engine::domain::repository::neighbor_repository::NeighborRepository;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::master_data_error::MasterDataError;

type KuniLoadResult = (Vec<Daimyo>, Vec<Kuni>, HashMap<u32, KuniId>);
type MasterDataResult = (Vec<Daimyo>, Vec<Kuni>, InMemoryNeighborRepository);

#[derive(Debug, Deserialize)]
struct NeighborRecord {
    #[serde(rename = "ID1")]
    id1: u32,
    #[serde(rename = "ID2")]
    id2: u32,
}

#[derive(Debug, Deserialize)]
struct KuniRecord {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "名前")]
    _name: String,
    #[serde(rename = "初期大名")]
    initial_daimyo: String,
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

pub struct InMemoryNeighborRepository {
    adjacency_map: HashMap<KuniId, Vec<KuniId>>,
}

impl InMemoryNeighborRepository {
    pub fn new(adjacency_map: HashMap<KuniId, Vec<KuniId>>) -> Self {
        Self { adjacency_map }
    }
}

impl NeighborRepository for InMemoryNeighborRepository {
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.adjacency_map.get(kuni_id).cloned().unwrap_or_default()
    }

    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool {
        self.adjacency_map
            .get(a)
            .is_some_and(|neighbors| neighbors.contains(b))
    }
}

pub struct MasterDataLoader;

impl MasterDataLoader {
    pub fn load(base_dir: &Path) -> Result<MasterDataResult, MasterDataError> {
        let (daimyos, kunis, id_map) = Self::load_kuni(base_dir)?;
        let neighbor_repo = Self::load_neighbor(base_dir, &id_map)?;
        Ok((daimyos, kunis, neighbor_repo))
    }

    fn load_kuni(base_dir: &Path) -> Result<KuniLoadResult, MasterDataError> {
        let kuni_csv_path = base_dir.join("kuni.csv");
        if !kuni_csv_path.exists() {
            return Err(MasterDataError::FileNotFound("kuni.csv".to_string()));
        }

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&kuni_csv_path)
            .map_err(|e| MasterDataError::Other(format!("Failed to open kuni.csv: {}", e)))?;

        let mut daimyo_map = HashMap::<String, Daimyo>::new();
        let mut kunis = Vec::new();
        let mut id_map = HashMap::<u32, KuniId>::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: KuniRecord = result.map_err(|e| {
                MasterDataError::ParseError {
                    line: i + 2,                  // header is line 1
                    field: "unknown".to_string(), // csv crate error doesn't always specify field easily
                    reason: e.to_string(),
                }
            })?;

            let daimyo_name = record.initial_daimyo.clone();
            let daimyo = daimyo_map
                .entry(daimyo_name.clone())
                .or_insert_with(|| Daimyo::new(DaimyoId::new(), daimyo_name));

            let kuni_id = KuniId::new();
            id_map.insert(record.id, kuni_id);

            let resource = Resource {
                kin: Amount::new(record.kin),
                hei: Amount::new(record.hei),
                kome: Amount::new(record.kome),
                jinko: Amount::new(record.jinko),
            };

            let stats = DevelopmentStats {
                kokudaka: Amount::new(record.kokudaka),
                machi: Amount::new(record.machi),
                tyu: Rate::new(record.tyu),
            };

            let kuni = Kuni::new(kuni_id, daimyo.id, resource, stats, IninFlag::new(false));
            kunis.push(kuni);
        }

        let daimyos: Vec<Daimyo> = daimyo_map.into_values().collect();
        Ok((daimyos, kunis, id_map))
    }

    fn load_neighbor(
        base_dir: &Path,
        id_map: &HashMap<u32, KuniId>,
    ) -> Result<InMemoryNeighborRepository, MasterDataError> {
        let neighbor_csv_path = base_dir.join("neighbor.csv");
        if !neighbor_csv_path.exists() {
            return Err(MasterDataError::FileNotFound("neighbor.csv".to_string()));
        }

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&neighbor_csv_path)
            .map_err(|e| MasterDataError::Other(format!("Failed to open neighbor.csv: {}", e)))?;

        let mut adjacency_map: HashMap<KuniId, Vec<KuniId>> = HashMap::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: NeighborRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "unknown".to_string(),
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

        Ok(InMemoryNeighborRepository::new(adjacency_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::domain::model::daimyo::DaimyoName;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_kuni_success() {
        let dir = tempdir().unwrap();
        let kuni_path = dir.path().join("kuni.csv");
        let mut file = File::create(kuni_path).unwrap();
        writeln!(file, "ID,名前,初期大名,金,兵,米,人口,石高,町,忠誠").unwrap();
        writeln!(file, "1,蝦夷,蛎崎,80,50,50,200,40,50,80").unwrap();
        writeln!(file, "2,奥州,伊達,120,70,60,260,80,60,70").unwrap();

        let (daimyos, kunis, id_map) = MasterDataLoader::load_kuni(dir.path()).unwrap();

        assert_eq!(daimyos.len(), 2);
        assert_eq!(kunis.len(), 2);
        assert_eq!(id_map.len(), 2);

        let ezo_id = id_map.get(&1).unwrap();
        let ezo = kunis.iter().find(|k| k.id == *ezo_id).unwrap();
        assert_eq!(ezo.resource.kin.value(), 80);

        let kakizaki = daimyos.iter().find(|d| d.id == ezo.daimyo_id).unwrap();
        assert_eq!(kakizaki.name, DaimyoName("蛎崎".to_string()));
    }

    #[test]
    fn test_load_kuni_parse_error() {
        let dir = tempdir().unwrap();
        let kuni_path = dir.path().join("kuni.csv");
        let mut file = File::create(kuni_path).unwrap();
        writeln!(file, "ID,名前,初期大名,金,兵,米,人口,石高,町,忠誠").unwrap();
        writeln!(file, "1,蝦夷,蛎崎,invalid_number,50,50,200,40,50,80").unwrap();

        let result = MasterDataLoader::load_kuni(dir.path());
        assert!(matches!(result, Err(MasterDataError::ParseError { .. })));
    }

    #[test]
    fn test_load_neighbor_success() {
        let dir = tempdir().unwrap();
        let kuni_path = dir.path().join("kuni.csv");
        let mut kuni_file = File::create(kuni_path).unwrap();
        writeln!(kuni_file, "ID,名前,初期大名,金,兵,米,人口,石高,町,忠誠").unwrap();
        writeln!(kuni_file, "1,A,A,0,0,0,0,0,0,0").unwrap();
        writeln!(kuni_file, "2,B,B,0,0,0,0,0,0,0").unwrap();
        writeln!(kuni_file, "3,C,C,0,0,0,0,0,0,0").unwrap();

        let (_, _, id_map) = MasterDataLoader::load_kuni(dir.path()).unwrap();

        let neighbor_path = dir.path().join("neighbor.csv");
        let mut neighbor_file = File::create(neighbor_path).unwrap();
        writeln!(neighbor_file, "ID1,ID2").unwrap();
        writeln!(neighbor_file, "1,2").unwrap();
        writeln!(neighbor_file, "2,3").unwrap();

        let neighbor_repo = MasterDataLoader::load_neighbor(dir.path(), &id_map).unwrap();

        let id1 = id_map.get(&1).unwrap();
        let id2 = id_map.get(&2).unwrap();
        let id3 = id_map.get(&3).unwrap();

        assert!(neighbor_repo.are_adjacent(id1, id2));
        assert!(neighbor_repo.are_adjacent(id2, id1)); // Bidirectional
        assert!(neighbor_repo.are_adjacent(id2, id3));
        assert!(!neighbor_repo.are_adjacent(id1, id3));
    }

    #[test]
    fn test_load_neighbor_invalid_reference() {
        let dir = tempdir().unwrap();
        let kuni_path = dir.path().join("kuni.csv");
        let mut kuni_file = File::create(kuni_path).unwrap();
        writeln!(kuni_file, "ID,名前,初期大名,金,兵,米,人口,石高,町,忠誠").unwrap();
        writeln!(kuni_file, "1,A,A,0,0,0,0,0,0,0").unwrap();

        let neighbor_path = dir.path().join("neighbor.csv");
        let mut neighbor_file = File::create(neighbor_path).unwrap();
        writeln!(neighbor_file, "ID1,ID2").unwrap();
        writeln!(neighbor_file, "1,999").unwrap(); // 999 does not exist in kuni.csv

        let result = MasterDataLoader::load(dir.path());
        assert!(matches!(
            result,
            Err(MasterDataError::InvalidReference { id: 999 })
        ));
    }
}
