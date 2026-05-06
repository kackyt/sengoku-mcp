use engine::domain::model::daimyo_personality::DaimyoPersonality;
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::*;
use engine::domain::service::cpu_action_decision_service::{CpuActionDecisionService, CpuActionDecision};
use engine::domain::model::resource::{Resource, DevelopmentStats};
use rand::thread_rng;

fn create_initial_kuni(id: u32, name: &str) -> Kuni {
    Kuni::new(
        KuniId(id),
        name.to_string(),
        DaimyoId(id),
        Resource {
            kin: Amount::from_display(DisplayAmount(1000)),
            kome: Amount::from_display(DisplayAmount(1000)),
            hei: Amount::from_display(DisplayAmount(100)),
            jinko: Amount::from_display(DisplayAmount(1000)),
        },
        DevelopmentStats {
            kokudaka: Amount::from_display(DisplayAmount(100)),
            machi: Amount::from_display(DisplayAmount(100)),
            tyu: Rate::new(50),
        },
        IninFlag(false),
    )
}

fn main() {
    let mut military_kuni = create_initial_kuni(1, "Military");
    let mut commerce_kuni = create_initial_kuni(2, "Commerce");
    let mut agriculture_kuni = create_initial_kuni(3, "Agriculture");

    let mil_p = DaimyoPersonality::new(0.5, 0.5, 2.0, 0.1);
    let com_p = DaimyoPersonality::new(0.5, 2.0, 0.5, 0.1);
    let agr_p = DaimyoPersonality::new(2.0, 0.5, 0.5, 0.1);

    let mut rng = thread_rng();

    println!("Starting 20-turn simulation...");
    println!("Turn | Mil(HEI/MCH/KKD) | Com(HEI/MCH/KKD) | Agr(HEI/MCH/KKD)");

    for t in 1..=20 {
        let turn = TurnNumber::new(t);
        
        // Military decision
        let (mil_dec, _) = CpuActionDecisionService::decide(&mil_p, &military_kuni, turn, &mut rng);
        apply_decision(&mut military_kuni, mil_dec);

        // Commerce decision
        let (com_dec, _) = CpuActionDecisionService::decide(&com_p, &commerce_kuni, turn, &mut rng);
        apply_decision(&mut commerce_kuni, com_dec);

        // Agriculture decision
        let (agr_dec, _) = CpuActionDecisionService::decide(&agr_p, &agriculture_kuni, turn, &mut rng);
        apply_decision(&mut agriculture_kuni, agr_dec);

        // Simple turn progression (income)
        if turn.season() == 0 { // Spring
            military_kuni.resource.kin += Amount::from_display(DisplayAmount(military_kuni.stats.machi.to_display().value() * 32 / 100));
            commerce_kuni.resource.kin += Amount::from_display(DisplayAmount(commerce_kuni.stats.machi.to_display().value() * 32 / 100));
            agriculture_kuni.resource.kin += Amount::from_display(DisplayAmount(agriculture_kuni.stats.machi.to_display().value() * 32 / 100));
        }
        if turn.season() == 2 { // Fall
            military_kuni.resource.kome += Amount::from_display(DisplayAmount(military_kuni.stats.kokudaka.to_display().value() * 32 / 100));
            commerce_kuni.resource.kome += Amount::from_display(DisplayAmount(commerce_kuni.stats.kokudaka.to_display().value() * 32 / 100));
            agriculture_kuni.resource.kome += Amount::from_display(DisplayAmount(agriculture_kuni.stats.kokudaka.to_display().value() * 32 / 100));
        }

        println!("{:>4} | {:>4}/{:>3}/{:>3} | {:>4}/{:>3}/{:>3} | {:>4}/{:>3}/{:>3}",
            t,
            military_kuni.resource.hei.to_display().value(), military_kuni.stats.machi.to_display().value(), military_kuni.stats.kokudaka.to_display().value(),
            commerce_kuni.resource.hei.to_display().value(), commerce_kuni.stats.machi.to_display().value(), commerce_kuni.stats.kokudaka.to_display().value(),
            agriculture_kuni.resource.hei.to_display().value(), agriculture_kuni.stats.machi.to_display().value(), agriculture_kuni.stats.kokudaka.to_display().value(),
        );
    }
}

fn apply_decision(kuni: &mut Kuni, dec: CpuActionDecision) {
    match dec {
        CpuActionDecision::DevelopLand { amount, .. } => {
            kuni.resource.kin -= Amount::from_display(amount);
            kuni.stats.kokudaka += Amount::from_display(DisplayAmount(amount.value() / 2));
        }
        CpuActionDecision::BuildTown { amount, .. } => {
            kuni.resource.kin -= Amount::from_display(amount);
            kuni.stats.machi += Amount::from_display(DisplayAmount(amount.value() / 2));
        }
        CpuActionDecision::Recruit { amount, .. } => {
            kuni.resource.kin -= Amount::from_display(DisplayAmount(amount.value() / 2));
            kuni.resource.jinko -= Amount::from_display(amount);
            kuni.resource.hei += Amount::from_display(amount);
        }
        CpuActionDecision::SellRice { amount, .. } => {
            kuni.resource.kome -= Amount::from_display(amount);
            kuni.resource.kin += Amount::from_display(DisplayAmount(amount.value() * 80 / 100));
        }
        CpuActionDecision::BuyRice { amount, .. } => {
            kuni.resource.kin -= Amount::from_display(amount);
            kuni.resource.kome += Amount::from_display(DisplayAmount(amount.value() * 80 / 100));
        }
        _ => {}
    }
}
