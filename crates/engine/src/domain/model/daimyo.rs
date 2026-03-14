use crate::domain::model::value_objects::DaimyoId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Daimyo {
    pub id: DaimyoId,
    pub name: String,
}

impl Daimyo {
    pub fn new(id: DaimyoId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}
