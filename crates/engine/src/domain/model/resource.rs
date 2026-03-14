use crate::domain::model::value_objects::{Amount, Rate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub kin: Amount,
    pub hei: Amount,
    pub kome: Amount,
    pub jinko: Amount,
}

impl Resource {
    pub fn new(kin: u32, hei: u32, kome: u32, jinko: u32) -> Self {
        Self {
            kin: Amount::new(kin),
            hei: Amount::new(hei),
            kome: Amount::new(kome),
            jinko: Amount::new(jinko),
        }
    }

    pub fn can_consume(&self, kin: u32, hei: u32, kome: u32) -> bool {
        self.kin.value() >= kin && self.hei.value() >= hei && self.kome.value() >= kome
    }

    pub fn consume(&mut self, kin: u32, hei: u32, kome: u32) -> Result<(), &'static str> {
        if !self.can_consume(kin, hei, kome) {
            return Err("Insufficient resources");
        }
        self.kin = self.kin.sub(Amount::new(kin));
        self.hei = self.hei.sub(Amount::new(hei));
        self.kome = self.kome.sub(Amount::new(kome));
        Ok(())
    }

    pub fn add(&mut self, kin: u32, hei: u32, kome: u32) {
        self.kin = self.kin.add(Amount::new(kin));
        self.hei = self.hei.add(Amount::new(hei));
        self.kome = self.kome.add(Amount::new(kome));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentStats {
    pub kokudaka: Amount,
    pub machi: Amount,
    pub tyu: Rate,
}

impl DevelopmentStats {
    pub fn new(kokudaka: u32, machi: u32, tyu: u32) -> Self {
        Self {
            kokudaka: Amount::new(kokudaka),
            machi: Amount::new(machi),
            tyu: Rate::new(tyu),
        }
    }
}
