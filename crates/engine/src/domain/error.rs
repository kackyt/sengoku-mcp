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
    InfrastructureError(#[from] anyhow::Error),

    /// その他のバリデーションエラーなど
    #[error("バリデーションエラー: {0}")]
    ValidationError(String),
}
