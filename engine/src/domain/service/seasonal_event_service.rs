use crate::domain::model::event::{SeasonalEventEffect, SeasonalEventType};
use crate::domain::model::kuni::{Kuni, ResourceSelector};
use crate::domain::model::value_objects::{Amount, TurnNumber, INTERNAL_SCALE};
use rand::Rng;

/// 季節イベントや資源生成のロジックを担当するドメインサービス
pub struct SeasonalEventService;

impl SeasonalEventService {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    /// ターン開始時に発生するイベント（洪水、疫病、反乱、資源生成、人口増加）を処理します
    pub fn process_start_turn_events(
        &self,
        turn: TurnNumber,
        kuni: &mut Kuni,
    ) -> Vec<SeasonalEventEffect> {
        let mut effects = Vec::new();
        let mut rng = rand::thread_rng();
        let season = (turn.value() - 1) % 4;

        // --- 1. 災害・反乱イベント ---

        // 疫病 (Plague): 通年 1/40
        if rng.gen_bool(1.0 / 40.0) {
            effects.push(self.trigger_plague(kuni));
        }

        // 洪水 (Flood): 夏 (season == 1) 1/40
        if season == 1 && rng.gen_bool(1.0 / 40.0) {
            effects.push(self.trigger_flood(kuni));
        }

        // 反乱 (Rebellion): 忠誠度 < 50, 確率 (50 - 忠誠度)%
        let tyu = kuni.stats.tyu.value();
        if tyu < 50 {
            let prob = (50 - tyu) as f64 / 100.0;
            if rng.gen_bool(prob) {
                effects.push(self.trigger_rebellion(kuni));
            }
        }

        // --- 2. 定期イベント（資源生成・人口増加） ---

        // 人口増加 (Population Growth): 春 (season == 0)
        if season == 0 {
            effects.push(self.process_population_growth(kuni));
        }

        // 資源生成 (Resource Income)
        if season == 0 {
            // 春: 金
            effects.push(self.process_gold_income(kuni));
        } else if season == 2 {
            // 秋: 米
            effects.push(self.process_rice_income(kuni));
        }

        effects
    }

    /// ターン終了時に発生するイベントを処理します
    pub fn process_end_turn_events(
        &self,
        _turn: TurnNumber,
        _kuni: &mut Kuni,
    ) -> Vec<SeasonalEventEffect> {
        // 現在、ターン終了時のイベントはすべてターン開始時処理に移動しました
        Vec::new()
    }

    // --- 各イベントの詳細ロジック ---

    fn trigger_plague(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();
        let p_jinko = rng.gen_range(5..=20);
        let p_hei = rng.gen_range(15..=34);
        let p_tyu = rng.gen_range(15..=24);
        let p_machi = rng.gen_range(5..=14);
        let p_kokudaka = rng.gen_range(5..=14);

        let jinko_loss = kuni.apply_percentage_loss(ResourceSelector::Jinko, p_jinko);
        let hei_loss = kuni.apply_percentage_loss(ResourceSelector::Hei, p_hei);
        let machi_loss = kuni.apply_percentage_loss(ResourceSelector::Machi, p_machi);
        let kokudaka_loss = kuni.apply_percentage_loss(ResourceSelector::Kokudaka, p_kokudaka);

        let tyu_before = kuni.stats.tyu.value();
        kuni.apply_percentage_loss(ResourceSelector::Tyu, p_tyu);
        let tyu_after = kuni.stats.tyu.value();
        let tyu_diff = -(tyu_before.saturating_sub(tyu_after) as i32);

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::Plague,
            kin_diff: Amount::zero(),
            kome_diff: Amount::zero(),
            hei_diff: hei_loss,
            jinko_diff: jinko_loss,
            tyu_diff,
            kokudaka_diff: kokudaka_loss,
            machi_diff: machi_loss,
        }
    }

    fn trigger_flood(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();
        let p_jinko = rng.gen_range(5..=10);
        let p_kome = rng.gen_range(10..=29);
        let p_tyu = rng.gen_range(20..=29);
        let p_machi = rng.gen_range(20..=39);
        let p_kokudaka = rng.gen_range(20..=39);

        let jinko_loss = kuni.apply_percentage_loss(ResourceSelector::Jinko, p_jinko);
        let kome_loss = kuni.apply_percentage_loss(ResourceSelector::Kome, p_kome);
        let machi_loss = kuni.apply_percentage_loss(ResourceSelector::Machi, p_machi);
        let kokudaka_loss = kuni.apply_percentage_loss(ResourceSelector::Kokudaka, p_kokudaka);

        let tyu_before = kuni.stats.tyu.value();
        kuni.apply_percentage_loss(ResourceSelector::Tyu, p_tyu);
        let tyu_after = kuni.stats.tyu.value();
        let tyu_diff = -(tyu_before.saturating_sub(tyu_after) as i32);

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::Flood,
            kin_diff: Amount::zero(),
            kome_diff: kome_loss,
            hei_diff: Amount::zero(),
            jinko_diff: jinko_loss,
            tyu_diff,
            kokudaka_diff: kokudaka_loss,
            machi_diff: machi_loss,
        }
    }

    fn trigger_rebellion(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();
        let p_hei = rng.gen_range(30..=50);
        let p_jinko = rng.gen_range(10..=20);
        let p_tyu = rng.gen_range(10..=20);
        let p_machi = rng.gen_range(10..=20);
        let p_kokudaka = rng.gen_range(10..=20);

        let hei_loss = kuni.apply_percentage_loss(ResourceSelector::Hei, p_hei);
        let jinko_loss = kuni.apply_percentage_loss(ResourceSelector::Jinko, p_jinko);
        let machi_loss = kuni.apply_percentage_loss(ResourceSelector::Machi, p_machi);
        let kokudaka_loss = kuni.apply_percentage_loss(ResourceSelector::Kokudaka, p_kokudaka);

        let tyu_before = kuni.stats.tyu.value();
        kuni.apply_percentage_loss(ResourceSelector::Tyu, p_tyu);
        let tyu_after = kuni.stats.tyu.value();
        let tyu_diff = -(tyu_before.saturating_sub(tyu_after) as i32);

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::Rebellion,
            kin_diff: Amount::zero(),
            kome_diff: Amount::zero(),
            hei_diff: hei_loss,
            jinko_diff: jinko_loss,
            tyu_diff,
            kokudaka_diff: kokudaka_loss,
            machi_diff: machi_loss,
        }
    }

    fn process_population_growth(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();
        let p_growth = rng.gen_range(10..=12);

        let growth = kuni.resource.jinko.mul_percent(p_growth);
        kuni.resource.jinko += growth;

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::PopulationGrowth,
            kin_diff: Amount::zero(),
            kome_diff: Amount::zero(),
            hei_diff: Amount::zero(),
            jinko_diff: growth,
            tyu_diff: 0,
            kokudaka_diff: Amount::zero(),
            machi_diff: Amount::zero(),
        }
    }

    fn process_gold_income(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();

        // 忠誠度の3～5%
        let tyu_rate = rng.gen_range(3..=5);
        let tyu_income = Amount::new(kuni.stats.tyu.value() * tyu_rate * INTERNAL_SCALE / 100);

        // 人口の10～15%
        let jinko_rate = rng.gen_range(10..=15);
        let jinko_income = kuni.resource.jinko.mul_percent(jinko_rate);

        // 町の25～40%
        let machi_rate = rng.gen_range(25..=40);
        let machi_income = kuni.stats.machi.mul_percent(machi_rate);

        let total_income = tyu_income.add(jinko_income).add(machi_income);
        kuni.resource.kin += total_income;

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::GoldIncome,
            kin_diff: total_income,
            kome_diff: Amount::zero(),
            hei_diff: Amount::zero(),
            jinko_diff: Amount::zero(),
            tyu_diff: 0,
            kokudaka_diff: Amount::zero(),
            machi_diff: Amount::zero(),
        }
    }

    fn process_rice_income(&self, kuni: &mut Kuni) -> SeasonalEventEffect {
        let mut rng = rand::thread_rng();

        // 忠誠度の3～5%
        let tyu_rate = rng.gen_range(3..=5);
        let tyu_income = Amount::new(kuni.stats.tyu.value() * tyu_rate * INTERNAL_SCALE / 100);

        // 人口の10～15%
        let jinko_rate = rng.gen_range(10..=15);
        let jinko_income = kuni.resource.jinko.mul_percent(jinko_rate);

        // 石高の25～40%
        let kokudaka_rate = rng.gen_range(25..=40);
        let kokudaka_income = kuni.stats.kokudaka.mul_percent(kokudaka_rate);

        let total_income = tyu_income.add(jinko_income).add(kokudaka_income);
        kuni.resource.kome += total_income;

        SeasonalEventEffect {
            kuni_id: kuni.id,
            event_type: SeasonalEventType::RiceIncome,
            kin_diff: Amount::zero(),
            kome_diff: total_income,
            hei_diff: Amount::zero(),
            jinko_diff: Amount::zero(),
            tyu_diff: 0,
            kokudaka_diff: Amount::zero(),
            machi_diff: Amount::zero(),
        }
    }
}
