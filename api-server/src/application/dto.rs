use chrono::{DateTime, Utc};
use serde::Serialize;

/// 自国の状況（REST API のレスポンス本体）
#[derive(Debug, Clone, Serialize)]
pub struct MyStatusDto {
    /// セッションID
    pub session_id: String,
    /// セッションの最終更新日時（MCP側で最後に保存された時刻）
    pub last_accessed_at: DateTime<Utc>,
    /// プレイヤーが選択している大名
    pub daimyo: DaimyoDto,
    /// ゲーム全体の進行状況
    pub game: GameProgressDto,
    /// 自領の一覧
    pub kunis: Vec<KuniStatusDto>,
    /// 自領全体の資源合計
    pub totals: ResourceTotalsDto,
    /// 自領に攻め込まれている合戦の一覧
    pub defense_alerts: Vec<DefenseAlertDto>,
}

/// 大名の概要
#[derive(Debug, Clone, Serialize)]
pub struct DaimyoDto {
    pub id: u32,
    pub name: String,
}

/// ゲームの進行状況
#[derive(Debug, Clone, Serialize)]
pub struct GameProgressDto {
    /// 現在のターン
    pub turn: u32,
    /// 季節名
    pub season: String,
    /// 進行フェーズ（Domestic / Battle / GameOver / GameClear）
    pub phase: String,
    /// 現在手番の大名名
    pub current_daimyo_name: String,
    /// 勝者の大名名（決着していない場合は `None`）
    pub winner: Option<String>,
}

/// 自領1国分の状況
#[derive(Debug, Clone, Serialize)]
pub struct KuniStatusDto {
    pub id: u32,
    pub name: String,
    /// 金
    pub kin: u32,
    /// 米
    pub kome: u32,
    /// 兵
    pub hei: u32,
    /// 人口
    pub jinko: u32,
    /// 石高
    pub kokudaka: u32,
    /// 町
    pub machi: u32,
    /// 忠誠度
    pub tyu: u32,
}

/// 自領全体の資源合計
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourceTotalsDto {
    /// 領地数
    pub kuni_count: usize,
    pub kin: u32,
    pub kome: u32,
    pub hei: u32,
    pub jinko: u32,
    pub kokudaka: u32,
}

/// 侵攻検知の情報
#[derive(Debug, Clone, Serialize)]
pub struct DefenseAlertDto {
    pub attacker_kuni_name: String,
    pub defender_kuni_name: String,
    pub enemy_hei: u32,
}
