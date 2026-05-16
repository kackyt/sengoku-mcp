use thiserror::Error;

/// ドメイン層で発生するエラーを定義します。
#[derive(Debug, Error)]
pub enum DomainError {
    /// リソースが不足している場合のエラー
    #[error("不足リソース: {0}")]
    InsufficientResource(String),

    /// 指定された対象が見つからない場合のエラー
    #[error("対象が見つかりません: {0}")]
    NotFound(String),

    /// 不正な操作が行われた場合のエラー
    #[error("不正な操作です: {0}")]
    InvalidOperation(String),

    /// 永続化層などのインフラストラクチャ由来のエラー
    #[error("内部エラー: {0}")]
    InfrastructureError(String),

    /// その他のバリデーションエラーなど
    #[error("バリデーションエラー: {0}")]
    ValidationError(String),

    /// 隣接していない操作を行った場合のエラー
    #[error("対象は隣接していません")]
    NotAdjacent,

    /// 現在の手番ではない場合のエラー
    #[error("現在の手番ではありません（現在の手番の国: {0:?}）")]
    NotYourTurn(crate::domain::model::value_objects::KuniId),
    /// 無効な戦術が指定された場合のエラー
    #[error("無効な戦術です (ID: {tactic_id}, 攻撃側: {is_attacker})")]
    InvalidTactic { tactic_id: u32, is_attacker: bool },
}
