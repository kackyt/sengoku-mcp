use crate::domain::model::value_objects::KuniId;

/// 国の隣接関係を管理するリポジトリのインターフェース
pub trait NeighborRepository: Send + Sync {
    /// 指定した国に隣接するすべての国のIDを取得します
    fn get_neighbors(&self, kuni_id: &KuniId) -> Vec<KuniId>;

    /// 2つの国が隣接しているかどうかを判定します
    fn are_adjacent(&self, a: &KuniId, b: &KuniId) -> bool;
}
