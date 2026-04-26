use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{AddAssign, SubAssign};

/// 表示用の金額、人数、量などを表す単位（整数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DisplayAmount(pub u32);

impl DisplayAmount {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// 内部単位の Amount に変換します
    pub fn to_internal(&self) -> Amount {
        Amount(self.0 * INTERNAL_SCALE)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for DisplayAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 内部計算用の金額、人数、量などを表す基本単位。
/// PRDで定義される BIAS (100倍) を内部スケールとして使用します。
pub const INTERNAL_SCALE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Amount(pub u32);

impl Amount {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// 表示用の単位 DisplayAmount に変換します（端数は切り捨て）
    pub fn to_display(&self) -> DisplayAmount {
        DisplayAmount(self.0 / INTERNAL_SCALE)
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn as_i32(&self) -> i32 {
        self.0 as i32
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

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 割合（0-100%）を表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rate(pub u32);

impl Rate {
    pub fn new(val: u32) -> Self {
        Self(val.min(100))
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn add(&self, other: Rate) -> Self {
        Self((self.0 + other.0).min(100))
    }

    pub fn sub(&self, other: Rate) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl AddAssign for Rate {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = (self.0 + rhs.0).min(100);
    }
}

impl SubAssign for Rate {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

/// 委任状態を表す型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// 国名を表す型
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KuniName(pub String);

/// 大名の識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventMessage(pub String);

impl EventMessage {
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
