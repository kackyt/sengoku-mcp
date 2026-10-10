//! REST API の入出力DTO
//!
//! 各構造体・フィールドのドキュメントコメントは、そのまま OpenAPI スキーマの説明として出力されます。

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// 自国の状況
///
/// 返すのは「自国の状況」「ターン数」「他国がどの大名の領地か」のみです。
/// 他国の資源（兵・金など）は含みません。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MyStatusDto {
    /// 現在のターン（1始まり）
    #[schema(example = 1)]
    pub turn: u32,
    /// プレイヤーが選択している大名（自国）
    pub daimyo: DaimyoDto,
    /// 自領の一覧（国ID順）
    pub my_kunis: Vec<KuniStatusDto>,
    /// 他国の一覧（国ID順）。どの大名の領地かのみを示す
    pub other_kunis: Vec<OtherKuniDto>,
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

/// 他国1国分の情報（支配大名のみ）
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OtherKuniDto {
    /// 国ID
    #[schema(example = 6)]
    pub id: u32,
    /// 国名
    #[schema(example = "三河")]
    pub name: String,
    /// この国を支配している大名
    pub daimyo: DaimyoDto,
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
