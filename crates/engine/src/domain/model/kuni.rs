use crate::domain::model::resource::{DevelopmentStats, Resource};
use crate::domain::model::value_objects::{Amount, DaimyoId, KuniId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kuni {
    pub id: KuniId,
    pub daimyo_id: DaimyoId,
    pub resource: Resource,
    pub stats: DevelopmentStats,
    pub inin: bool,
}

impl Kuni {
    pub fn new(
        id: KuniId,
        daimyo_id: DaimyoId,
        resource: Resource,
        stats: DevelopmentStats,
        inin: bool,
    ) -> Self {
        Self {
            id,
            daimyo_id,
            resource,
            stats,
            inin,
        }
    }

    pub fn set_daimyo_id(&mut self, daimyo_id: DaimyoId) {
        self.daimyo_id = daimyo_id;
    }

    pub fn set_inin(&mut self, inin: bool) {
        self.inin = inin;
    }

    pub fn add_resource(&mut self, kin: u32, hei: u32, kome: u32) {
        self.resource.add(kin, hei, kome);
    }

    pub fn consume_resource(&mut self, kin: u32, hei: u32, kome: u32) -> Result<(), &'static str> {
        self.resource.consume(kin, hei, kome)
    }

    pub fn modify_jinko(&mut self, delta: i32) {
        let current = self.resource.jinko.value() as i32;
        let next = (current + delta).max(0) as u32;
        self.resource.jinko = Amount::new(next);
    }

    pub fn modify_tyu(&mut self, delta: i32) {
        let current = self.stats.tyu.value() as i32;
        let next = (current + delta).clamp(0, 100) as u32;
        self.stats.tyu = crate::domain::model::value_objects::Rate::new(next);
    }
}
