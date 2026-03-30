use std::fmt;

/// 金額、人数、量などを表す基本単位。
/// PRDで定義される BIAS (10倍) を内部スケールとして使用します。
pub const INTERNAL_SCALE: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(pub u32);

impl Amount {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    /// 表示用の値（整数）から内部単位の Amount を作成します
    pub fn from_display(val: u32) -> Self {
        Self(val * INTERNAL_SCALE)
    }

    /// 内部単位の値を表示用の整数に変換します
    pub fn to_display(&self) -> u32 {
        self.0 / INTERNAL_SCALE
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn add(&self, other: Amount) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn sub(&self, other: Amount) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn mul_percent(&self, percent: u32) -> Self {
        Self((self.0 as u64 * percent as u64 / 100) as u32)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 割合（0-100%）を表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rate(pub u32);

impl Rate {
    pub fn new(val: u32) -> Self {
        Self(val.min(100))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// 委任状態を表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IninFlag(pub bool);

impl IninFlag {
    pub fn new(val: bool) -> Self {
        Self(val)
    }

    pub fn is_enabled(&self) -> bool {
        self.0
    }
}

/// ユニットの識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub uuid::Uuid);

impl Default for UnitId {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// 国の識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KuniId(pub uuid::Uuid);

impl Default for KuniId {
    fn default() -> Self {
        Self::new()
    }
}

impl KuniId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// 大名の識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DaimyoId(pub uuid::Uuid);

impl Default for DaimyoId {
    fn default() -> Self {
        Self::new()
    }
}

impl DaimyoId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// 経過ターン数を表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TurnNumber(pub u32);

impl TurnNumber {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// 行動順のインデックスを表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionOrderIndex(pub usize);

impl ActionOrderIndex {
    pub fn new(val: usize) -> Self {
        Self(val)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

/// イベントの詳細やログメッセージを表す型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventMessage(pub String);

impl EventMessage {
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
