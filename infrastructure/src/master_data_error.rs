use thiserror::Error;

/// マスターデータ読み込みに関するエラー
#[derive(Debug, Error)]
pub enum MasterDataError {
    /// 指定されたマスターデータファイルが見つからない場合のエラー
    #[error("マスターデータファイルが見つかりません: {0}")]
    FileNotFound(String),

    /// CSVファイルのパースに失敗した場合のエラー
    #[error("パースエラー (行: {line}, フィールド: {field}): {reason}")]
    ParseError {
        line: usize,
        field: String,
        reason: String,
    },

    /// 存在しないIDなどを参照している場合のエラー
    #[error("無効な参照ID: {id}")]
    InvalidReference { id: u32 },

    /// I/O操作中に発生したエラー
    #[error("I/Oエラー: {0}")]
    IoError(#[from] std::io::Error),

    /// CSV操作中に発生したエラー
    #[error("CSVエラー: {0}")]
    CsvError(#[from] csv::Error),

    /// その他の予期せぬエラー
    #[error("その他のエラー: {0}")]
    Other(String),
}
