/// 大名の行動傾向を定義する性格パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct DaimyoPersonality {
    /// 農業志向 (高いほど開墾・収穫重視)
    pub agriculture_bias: f64,
    /// 商業志向 (高いほど町造り・金収入重視)
    pub commerce_bias: f64,
    /// 軍事志向 (高いほど徴募・戦争重視)
    pub military_bias: f64,
    /// 性格の揺らぎ幅 (0.0=ランダムなし, 1.0=最大揺れる)
    pub randomness: f64,
}

impl DaimyoPersonality {
    pub fn new(
        agriculture_bias: f64,
        commerce_bias: f64,
        military_bias: f64,
        randomness: f64,
    ) -> Self {
        Self {
            agriculture_bias,
            commerce_bias,
            military_bias,
            randomness,
        }
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
