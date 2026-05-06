use crate::domain::model::{daimyo_personality::DaimyoPersonality, value_objects::DaimyoId};

/// 大名の名前を表す型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DaimyoName(pub String);

/// 大名を表すドメインモデル
#[derive(Debug, Clone, PartialEq)]
pub struct Daimyo {
    /// 大名ID
    pub id: DaimyoId,
    /// 大名名
    pub name: DaimyoName,
    /// 性格パラメータ
    pub personality: DaimyoPersonality,
}

impl Daimyo {
    /// 新しい大名を作成します
    pub fn new(id: DaimyoId, name: impl Into<String>, personality: DaimyoPersonality) -> Self {
        Self {
            id,
            name: DaimyoName(name.into()),
            personality,
        }
    }
}
