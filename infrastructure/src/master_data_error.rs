use thiserror::Error;

#[derive(Debug, Error)]
pub enum MasterDataError {
    #[error("マスターデータファイルが見つかりません: {0}")]
    FileNotFound(String),
    #[error("パースエラー (行: {line}, フィールド: {field}): {reason}")]
    ParseError {
        line: usize,
        field: String,
        reason: String,
    },
    #[error("無効な参照ID: {id}")]
    InvalidReference { id: u32 },
    #[error("その他のエラー: {0}")]
    Other(String),
}
