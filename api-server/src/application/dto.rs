//! REST API の入出力DTO
//!
//! 各構造体・フィールドのドキュメントコメントは、そのまま OpenAPI スキーマの説明として出力されます。

use chrono::{DateTime, Utc};
use engine::domain::model::game_state::GamePhase;
use serde::Serialize;
use utoipa::ToSchema;

/// 自国の状況
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MyStatusDto {
    /// セッションの最終更新日時（MCP側で最後に保存された時刻, UTC）
    pub last_accessed_at: DateTime<Utc>,
    /// プレイヤーが選択している大名
    pub daimyo: DaimyoDto,
    /// ゲーム全体の進行状況
    pub game: GameProgressDto,
    /// 自領の一覧（国ID順）
    pub kunis: Vec<KuniStatusDto>,
    /// 自領全体の資源合計
    pub totals: ResourceTotalsDto,
    /// 自領に攻め込まれている合戦の一覧（空なら侵攻なし）
    pub defense_alerts: Vec<DefenseAlertDto>,
}

/// 大名の概要
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DaimyoDto {
    /// 大名ID
    #[schema(example = 7)]
    pub id: u32,
    /// 大名名
    #[schema(example = "織田")]
    pub name: String,
}

/// ゲームの進行フェーズ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub enum GamePhaseDto {
    /// 内政フェーズ
    Domestic,
    /// 合戦フェーズ
    Battle,
    /// ゲームオーバー（自国が滅亡）
    GameOver,
    /// 天下一統（クリア）
    GameClear,
}

impl From<GamePhase> for GamePhaseDto {
    fn from(phase: GamePhase) -> Self {
        match phase {
            GamePhase::Domestic => Self::Domestic,
            GamePhase::Battle => Self::Battle,
            GamePhase::GameOver => Self::GameOver,
            GamePhase::GameClear => Self::GameClear,
        }
    }
}

/// ゲームの進行状況
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GameProgressDto {
    /// 現在のターン（1始まり）
    #[schema(example = 1)]
    pub turn: u32,
    /// 季節名（春 / 夏 / 秋 / 冬）
    #[schema(example = "春")]
    pub season: String,
    /// 進行フェーズ
    pub phase: GamePhaseDto,
    /// 現在手番の大名名
    #[schema(example = "織田")]
    pub current_daimyo_name: String,
    /// 勝者の大名名（決着していない場合は null）
    pub winner: Option<String>,
}

/// 自領1国分の状況
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KuniStatusDto {
    /// 国ID
    #[schema(example = 7)]
    pub id: u32,
    /// 国名
    #[schema(example = "尾張")]
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
    /// 忠誠度（0〜100）
    pub tyu: u32,
}

/// 自領全体の資源合計
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ResourceTotalsDto {
    /// 領地数
    pub kuni_count: usize,
    /// 金の合計
    pub kin: u32,
    /// 米の合計
    pub kome: u32,
    /// 兵の合計
    pub hei: u32,
    /// 人口の合計
    pub jinko: u32,
    /// 石高の合計
    pub kokudaka: u32,
}

/// 侵攻検知の情報（自領が攻め込まれている合戦）
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DefenseAlertDto {
    /// 攻め込んできた国の名前
    pub attacker_kuni_name: String,
    /// 攻め込まれている自領の名前
    pub defender_kuni_name: String,
    /// 敵軍の兵数
    pub enemy_hei: u32,
}

/// Webで作成したゲームの情報
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedGameDto {
    /// 状況取得用の閲覧トークン（32桁の16進数）。ブラウザで保持してください。
    #[schema(example = "777fc6d170d94495b49babeba2230e7d")]
    pub view_token: String,
    /// 状況取得APIのパス（`GET` でポーリングする）
    #[schema(example = "/api/views/777fc6d170d94495b49babeba2230e7d/status")]
    pub status_url: String,
    /// チャット（LLM）に伝える参加コード（6文字・1回限り）
    #[schema(example = "VQ4X7K")]
    pub join_code: String,
    /// 参加コードの有効期限（UTC）
    pub join_code_expires_at: DateTime<Utc>,
    /// プレイヤー向けの案内文
    #[schema(example = "チャットで「参加コード VQ4X7K でゲームに参加して」と伝えてください")]
    pub join_message: String,
}
