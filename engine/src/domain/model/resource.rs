use crate::domain::model::value_objects::{Amount, Rate};

/// 国の資源（金、兵、米、人口）を管理する構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// 所持金
    pub kin: Amount,
    /// 兵数
    pub hei: Amount,
    /// 備蓄米
    pub kome: Amount,
    /// 人口
    pub jinko: Amount,
}

impl Resource {
    /// 新しい資源セットを作成します
    pub fn new(kin: u32, hei: u32, kome: u32, jinko: u32) -> Self {
        Self {
            kin: Amount::new(kin),
            hei: Amount::new(hei),
            kome: Amount::new(kome),
            jinko: Amount::new(jinko),
        }
    }

    /// 指定された資源を消費可能かチェックします
    pub fn can_consume(&self, kin: Amount, hei: Amount, kome: Amount, jinko: Amount) -> bool {
        self.kin >= kin && self.hei >= hei && self.kome >= kome && self.jinko >= jinko
    }

    /// 資源を消費します。不足している場合はエラーを返します。
    pub fn consume(
        &mut self,
        kin: Amount,
        hei: Amount,
        kome: Amount,
        jinko: Amount,
    ) -> Result<(), &'static str> {
        if !self.can_consume(kin, hei, kome, jinko) {
            return Err("Insufficient resources");
        }
        self.kin -= kin;
        self.hei -= hei;
        self.kome -= kome;
        self.jinko -= jinko;
        Ok(())
    }

    /// 資源を追加します
    pub fn add(&mut self, kin: Amount, hei: Amount, kome: Amount, jinko: Amount) {
        self.kin += kin;
        self.hei += hei;
        self.kome += kome;
        self.jinko += jinko;
    }
}

/// 国の開発状況（石高、町、忠誠度）を管理する構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentStats {
    /// 石高（農業生産力）
    pub kokudaka: Amount,
    /// 町ランク（商業発展度）
    pub machi: Amount,
    /// 国民の忠誠度
    pub tyu: Rate,
}

impl DevelopmentStats {
    /// 新しい開発統計を作成します
    pub fn new(kokudaka: u32, machi: u32, tyu: u32) -> Self {
        Self {
            kokudaka: Amount::new(kokudaka),
            machi: Amount::new(machi),
            tyu: Rate::new(tyu),
        }
    }
}
