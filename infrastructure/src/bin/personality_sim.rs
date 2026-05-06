use clap::Parser;
use engine::domain::service::simulation_service::SimulationService;
use engine::domain::{
    model::{
        daimyo::Daimyo,
        daimyo_personality::DaimyoPersonality,
        kuni::Kuni,
        resource::{DevelopmentStats, Resource},
        value_objects::*,
    },
    service::cpu_action_decision_service::CpuActionDecisionService,
};
use infrastructure::master_data::MasterDataLoader;
use rand::thread_rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 3年間の全国シミュレーションを行う
    #[arg(short, long)]
    master: bool,

    /// シミュレーションするターン数
    #[arg(short, long, default_value_t = 12)]
    turns: u32,
}

fn create_initial_kuni(id: u32, name: &str) -> Kuni {
    Kuni::new(
        KuniId::new(id),
        name.to_string(),
        DaimyoId::new(id),
        Resource {
            kin: DisplayAmount::new(1000).to_internal(),
            kome: DisplayAmount::new(1000).to_internal(),
            hei: DisplayAmount::new(100).to_internal(),
            jinko: DisplayAmount::new(1000).to_internal(),
        },
        DevelopmentStats {
            kokudaka: DisplayAmount::new(100).to_internal(),
            machi: DisplayAmount::new(100).to_internal(),
            tyu: Rate::new(50),
        },
        IninFlag::new(false),
    )
}

fn run_preset_simulation(turns: u32) {
    let mil_p = DaimyoPersonality::new(0.5, 0.5, 2.0, 0.1);
    let com_p = DaimyoPersonality::new(0.5, 2.0, 0.5, 0.1);
    let agr_p = DaimyoPersonality::new(2.0, 0.5, 0.5, 0.1);

    let daimyos = vec![
        Daimyo::new(DaimyoId::new(1), "Military", mil_p),
        Daimyo::new(DaimyoId::new(2), "Commerce", com_p),
        Daimyo::new(DaimyoId::new(3), "Agriculture", agr_p),
    ];

    let kunis = vec![
        create_initial_kuni(1, "MilCountry"),
        create_initial_kuni(2, "ComCountry"),
        create_initial_kuni(3, "AgrCountry"),
    ];

    let mut rng = thread_rng();

    println!("Starting {}-turn PRESET simulation...", turns);
    println!("Turn | Mil(HEI/MCH/KKD) | Com(HEI/MCH/KKD) | Agr(HEI/MCH/KKD)");
    println!("{:-<70}", "");

    let snapshots = SimulationService::run_simulation(&daimyos, &kunis, turns, &mut rng);

    for snapshot in snapshots {
        let t = snapshot.turn.value();
        let mil = &snapshot.kuni_states[0];
        let com = &snapshot.kuni_states[1];
        let agr = &snapshot.kuni_states[2];

        println!(
            "{:>4} | {:>4}/{:>3}/{:>3} | {:>4}/{:>3}/{:>3} | {:>4}/{:>3}/{:>3}",
            t,
            mil.resource.hei.to_display().value(),
            mil.stats.machi.to_display().value(),
            mil.stats.kokudaka.to_display().value(),
            com.resource.hei.to_display().value(),
            com.stats.machi.to_display().value(),
            com.stats.kokudaka.to_display().value(),
            agr.resource.hei.to_display().value(),
            agr.stats.machi.to_display().value(),
            agr.stats.kokudaka.to_display().value(),
        );
    }
}

fn run_master_data_simulation(turns: u32) {
    let bundle = MasterDataLoader::load().expect("Failed to load master data");
    let initial_kunis = bundle.kunis;
    let daimyos = bundle.daimyos;
    let mut daimyo_map = HashMap::new();
    for d in &daimyos {
        daimyo_map.insert(d.id, d.clone());
    }

    // 決定論的なシミュレーションのためにシードを固定
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    println!("\n=== Master Data Simulation ({} Turns) ===", turns);

    let snapshots = SimulationService::run_simulation(&daimyos, &initial_kunis, turns, &mut rng);
    let final_snapshot = snapshots.last().unwrap();

    println!("\n=== Final State After {} Turns ===", turns);
    println!(
        "{:<10} | {:<8} | {:<5} | {:<5} | {:<5} | {:<5} | {:<5} | {:<5} | {:<6}",
        "Daimyo", "Kokudaka", "Machi", "Hei", "Kin", "Kome", "Jinko", "Tyu", "Score"
    );
    println!("{:-<80}", "");

    for kuni in &final_snapshot.kuni_states {
        let daimyo = daimyo_map.get(&kuni.daimyo_id).unwrap();
        println!(
            "{:<10} | {:>8} | {:>5} | {:>5} | {:>5} | {:>5} | {:>5} | {:>5} | {:>6}",
            daimyo.name.0,
            kuni.stats.kokudaka.to_display().value(),
            kuni.stats.machi.to_display().value(),
            kuni.resource.hei.to_display().value(),
            kuni.resource.kin.to_display().value(),
            kuni.resource.kome.to_display().value(),
            kuni.resource.jinko.to_display().value(),
            kuni.stats.tyu.value(),
            CpuActionDecisionService::evaluate_score(kuni, TurnNumber::new(turns))
        );
    }
}

fn main() {
    let args = Args::parse();

    if args.master {
        run_master_data_simulation(args.turns);
    } else {
        run_preset_simulation(args.turns);
        println!("\nTip: Use '--master' flag to run simulation with real master data.");
    }
}
