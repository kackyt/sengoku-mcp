use crate::domain::error::DomainError;

/// 大名の行動傾向を定義する性格パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct DaimyoPersonality {
    /// 農業志向 (高いほど開墾・収穫重視)
    agriculture_bias: f64,
    /// 商業志向 (高いほど町造り・金収入重視)
    commerce_bias: f64,
    /// 軍事志向 (高いほど徴募・戦争重視)
    military_bias: f64,
    /// 性格の揺らぎ幅 (0.0=ランダムなし, 1.0=最大揺れる)
    randomness: f64,
}

impl DaimyoPersonality {
    pub fn new(
        agriculture_bias: f64,
        commerce_bias: f64,
        military_bias: f64,
        randomness: f64,
    ) -> Result<Self, DomainError> {
        let vals = [agriculture_bias, commerce_bias, military_bias, randomness];
        if vals.iter().any(|v| !v.is_finite()) {
            return Err(DomainError::ValidationError(
                "性格パラメータに非有限値が含まれています".into(),
            ));
        }
        if agriculture_bias < 0.0 || commerce_bias < 0.0 || military_bias < 0.0 {
            return Err(DomainError::ValidationError(
                "志向値（bias）は0以上である必要があります".into(),
            ));
        }
        if !(0.0..=1.0).contains(&randomness) {
            return Err(DomainError::ValidationError(
                "randomnessは0.0から1.0の範囲である必要があります".into(),
            ));
        }

        Ok(Self {
            agriculture_bias,
            commerce_bias,
            military_bias,
            randomness,
        })
    }

    pub fn agriculture_bias(&self) -> f64 {
        self.agriculture_bias
    }

    pub fn commerce_bias(&self) -> f64 {
        self.commerce_bias
    }

    pub fn military_bias(&self) -> f64 {
        self.military_bias
    }

    pub fn randomness(&self) -> f64 {
        self.randomness
    }
}

impl Default for DaimyoPersonality {
    fn default() -> Self {
        Self {
            agriculture_bias: 1.0,
            commerce_bias: 1.0,
            military_bias: 1.0,
            randomness: 0.2,
        }
    }
}
