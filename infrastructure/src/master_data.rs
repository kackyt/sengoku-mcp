use engine::domain::model::daimyo::Daimyo;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::resource::{DevelopmentStats, Resource};
use engine::domain::model::value_objects::{Amount, DaimyoId, IninFlag, KuniId, Rate};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::master_data_error::MasterDataError;
use crate::persistence::InMemoryNeighborRepository;

/// マスターデータのロード結果をまとめた構造体
pub struct MasterDataBundle {
    /// ロードされた大名のリスト
    pub daimyos: Vec<Daimyo>,
    /// ロードされた国のリスト
    pub kunis: Vec<Kuni>,
    /// ロードされた隣接情報を持つリポジトリ
    pub neighbor_repo: InMemoryNeighborRepository,
}

/// 国データのロード結果
struct KuniLoadResult {
    /// 大名のリスト
    daimyos: Vec<Daimyo>,
    /// 国のリスト
    kunis: Vec<Kuni>,
    /// CSV上のIDから内部IDへのマッピング
    id_map: HashMap<u32, KuniId>,
}

/// 隣接情報のCSVレコード
#[derive(Debug, Deserialize)]
struct NeighborRecord {
    /// 国ID1
    #[serde(rename = "ID1")]
    id1: u32,
    /// 国ID2
    #[serde(rename = "ID2")]
    id2: u32,
}

/// 国情報のCSVレコード
#[derive(Debug, Deserialize)]
struct KuniRecord {
    /// 国ID (CSV内)
    #[serde(rename = "ID")]
    id: u32,
    /// 国名
    #[serde(rename = "名前")]
    name: String,
    /// 初期大名名
    #[serde(rename = "初期大名")]
    initial_daimyo: String,
    /// 所持金
    #[serde(rename = "金")]
    kin: u32,
    /// 兵数
    #[serde(rename = "兵")]
    hei: u32,
    /// 米数
    #[serde(rename = "米")]
    kome: u32,
    /// 人口
    #[serde(rename = "人口")]
    jinko: u32,
    /// 石高
    #[serde(rename = "石高")]
    kokudaka: u32,
    /// 町数
    #[serde(rename = "町")]
    machi: u32,
    /// 忠誠度
    #[serde(rename = "忠誠")]
    tyu: u32,
}

/// CSVファイルからマスターデータを読み込むローダー
pub struct MasterDataLoader;

impl MasterDataLoader {
    /// ベースディレクトリからマスターデータを一括ロードする
    pub fn load(base_dir: &Path) -> Result<MasterDataBundle, MasterDataError> {
        let kuni_result = Self::load_kuni(base_dir)?;
        let neighbor_repo = Self::load_neighbor(base_dir, &kuni_result.id_map)?;

        Ok(MasterDataBundle {
            daimyos: kuni_result.daimyos,
            kunis: kuni_result.kunis,
            neighbor_repo,
        })
    }

    /// kuni.csv から大名と国のデータを読み込む
    fn load_kuni(base_dir: &Path) -> Result<KuniLoadResult, MasterDataError> {
        let kuni_csv_path = base_dir.join("kuni.csv");
        if !kuni_csv_path.exists() {
            return Err(MasterDataError::FileNotFound("kuni.csv".to_string()));
        }

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&kuni_csv_path)?; // ? により CsvError (内部に IoError を含む) へ自動変換される

        let mut daimyo_map = HashMap::<String, Daimyo>::new();
        let mut kunis = Vec::new();
        let mut id_map = HashMap::<u32, KuniId>::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: KuniRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2, // ヘッダー分を考慮
                field: "不明".to_string(),
                reason: e.to_string(),
            })?;

            // 大名の登録（既に存在すれば既存のものを使用）
            let daimyo_name = record.initial_daimyo;
            let daimyo = daimyo_map
                .entry(daimyo_name.clone()) // キーとして1回クローン
                .or_insert_with(|| Daimyo::new(DaimyoId::new(), daimyo_name)); // 存在しない場合のみそのまま使用

            let kuni_id = KuniId::new();
            id_map.insert(record.id, kuni_id);

            // 資源データの構築 (内部計算用に10倍する)
            let resource = Resource {
                kin: Amount::new(record.kin * 10),
                hei: Amount::new(record.hei * 10),
                kome: Amount::new(record.kome * 10),
                jinko: Amount::new(record.jinko * 10),
            };

            // 開発ステータスの構築 (内部計算用に10倍する)
            let stats = DevelopmentStats {
                kokudaka: Amount::new(record.kokudaka * 10),
                machi: Amount::new(record.machi * 10),
                tyu: Rate::new(record.tyu), // 忠誠度は%なのでそのまま
            };

            // 国エンティティの作成
            let kuni = Kuni::new(kuni_id, record.name, daimyo.id, resource, stats, IninFlag::new(false));
            kunis.push(kuni);
        }

        let daimyos: Vec<Daimyo> = daimyo_map.into_values().collect();
        Ok(KuniLoadResult {
            daimyos,
            kunis,
            id_map,
        })
    }

    /// neighbor.csv から隣接情報を読み込み、リポジトリを構築する
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
            .from_path(&neighbor_csv_path)?;

        let mut adjacency_map: HashMap<KuniId, Vec<KuniId>> = HashMap::new();

        for (i, result) in rdr.deserialize().enumerate() {
            let record: NeighborRecord = result.map_err(|e| MasterDataError::ParseError {
                line: i + 2,
                field: "不明".to_string(),
                reason: e.to_string(),
            })?;

            // CSV上のIDを内部IDに変換
            let id1 = id_map
                .get(&record.id1)
                .ok_or(MasterDataError::InvalidReference { id: record.id1 })?;
            let id2 = id_map
                .get(&record.id2)
                .ok_or(MasterDataError::InvalidReference { id: record.id2 })?;

            // 相互の隣接関係を登録
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
    use engine::domain::repository::neighbor_repository::NeighborRepository;
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

        let result = MasterDataLoader::load_kuni(dir.path()).unwrap();

        assert_eq!(result.daimyos.len(), 2);
        assert_eq!(result.kunis.len(), 2);
        assert_eq!(result.id_map.len(), 2);

        let ezo_id = result.id_map.get(&1).unwrap();
        let ezo = result.kunis.iter().find(|k| k.id == *ezo_id).unwrap();
        assert_eq!(ezo.resource.kin.value(), 80);

        let kakizaki = result
            .daimyos
            .iter()
            .find(|d| d.id == ezo.daimyo_id)
            .unwrap();
        assert_eq!(kakizaki.name, DaimyoName("蛎崎".to_string()));
    }

    #[test]
    fn test_load_kuni_parse_error() {
        let dir = tempdir().unwrap();
        let kuni_path = dir.path().join("kuni.csv");
        let mut file = File::create(kuni_path).unwrap();
        writeln!(file, "ID,名前,初期大名,金,兵,米,人口,石高,町,忠誠").unwrap();
        writeln!(file, "1,蝦夷,蛎崎,数値ではない,50,50,200,40,50,80").unwrap();

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

        let kuni_result = MasterDataLoader::load_kuni(dir.path()).unwrap();

        let neighbor_path = dir.path().join("neighbor.csv");
        let mut neighbor_file = File::create(neighbor_path).unwrap();
        writeln!(neighbor_file, "ID1,ID2").unwrap();
        writeln!(neighbor_file, "1,2").unwrap();
        writeln!(neighbor_file, "2,3").unwrap();

        let neighbor_repo =
            MasterDataLoader::load_neighbor(dir.path(), &kuni_result.id_map).unwrap();

        let id1 = kuni_result.id_map.get(&1).unwrap();
        let id2 = kuni_result.id_map.get(&2).unwrap();
        let id3 = kuni_result.id_map.get(&3).unwrap();

        assert!(neighbor_repo.are_adjacent(id1, id2));
        assert!(neighbor_repo.are_adjacent(id2, id1)); // 相互方向の確認
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
        writeln!(neighbor_file, "1,999").unwrap(); // 999 は kuni.csv に存在しないID

        let result = MasterDataLoader::load(dir.path());
        assert!(matches!(
            result,
            Err(MasterDataError::InvalidReference { id: 999 })
        ));
    }
}
