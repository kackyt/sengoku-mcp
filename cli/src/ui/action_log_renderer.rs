use engine::domain::model::action_log::{ActionLogEvent, DomesticLogEvent, WarLogEvent};

pub fn render_event(event: &ActionLogEvent) -> String {
    match event {
        ActionLogEvent::Domestic(e) => match e {
            DomesticLogEvent::RiceSold {
                kuni_name, gain, ..
            } => format!("{}：米を売却し、金{}を得ました", kuni_name.0, gain.value()),
            DomesticLogEvent::RiceBought {
                kuni_name,
                cost,
                amount,
                ..
            } => format!(
                "{}：米を{}購入しました（金{}を消費）",
                kuni_name.0,
                amount.value(),
                cost.value()
            ),
            DomesticLogEvent::LandReclaimed {
                kuni_name, gain, ..
            } => format!(
                "{}：開墾し、石高が{}上昇しました",
                kuni_name.0,
                gain.value()
            ),
            DomesticLogEvent::TownDeveloped {
                kuni_name, gain, ..
            } => format!(
                "{}：町を整備し、町が{}上昇しました",
                kuni_name.0,
                gain.value()
            ),
            DomesticLogEvent::TroopsDrafted {
                kuni_name, amount, ..
            } => format!("{}：兵を{}徴募しました", kuni_name.0, amount.value()),
            DomesticLogEvent::TroopsDismissed {
                kuni_name, amount, ..
            } => format!("{}：兵を{}解雇しました", kuni_name.0, amount.value()),
            DomesticLogEvent::CharityPerformed {
                kuni_name,
                gain_tyu,
                ..
            } => format!(
                "{}：施しを行い、忠誠度が{}上昇しました",
                kuni_name.0,
                gain_tyu.value()
            ),
            DomesticLogEvent::ResourcesTransported {
                from_kuni,
                to_kuni,
                kin,
                hei,
                kome,
            } => format!(
                "{}→{}：資源を輸送しました（金:{} 兵:{} 米:{}）",
                from_kuni.0,
                to_kuni.0,
                kin.value(),
                hei.value(),
                kome.value()
            ),
            DomesticLogEvent::DelegationChanged { kuni_name, enabled } => format!(
                "{}：委任を{}にしました",
                kuni_name.0,
                if *enabled { "ON" } else { "OFF" }
            ),
            DomesticLogEvent::CpuAction {
                daimyo_id,
                action_msg,
            } => format!("CPU (Daimyo={:?}): {}", daimyo_id, action_msg),
            DomesticLogEvent::TurnStart { turn, season } => {
                let season_name = match season {
                    0 => "春",
                    1 => "夏",
                    2 => "秋",
                    _ => "冬",
                };
                format!("第{}ターン（{}）が始まりました", turn.value(), season_name)
            }
            DomesticLogEvent::SeasonalEvent {
                event_type,
                kuni_names,
            } => {
                use engine::domain::model::event::SeasonalEventType;
                let names = kuni_names.iter().map(|n| n.0.as_str()).collect::<Vec<_>>();
                match event_type {
                    SeasonalEventType::GoldIncome => {
                        "春の収穫：各地で金が徴収されました".to_string()
                    }
                    SeasonalEventType::RiceIncome => {
                        "秋の収穫：各地で米が増産されました".to_string()
                    }
                    SeasonalEventType::PopulationGrowth => {
                        "春の恵み：各地の人口が増加しました".to_string()
                    }
                    SeasonalEventType::Plague => {
                        format!("【疫病】疫病が発生しました：{}", names.join("、"))
                    }
                    SeasonalEventType::Flood => {
                        format!("【洪水】洪水に見舞われました：{}", names.join("、"))
                    }
                    SeasonalEventType::Rebellion => {
                        format!("【反乱】反乱が発生しました：{}", names.join("、"))
                    }
                }
            }
            DomesticLogEvent::WarStarted {
                attacker_name,
                defender_name,
            } => format!(
                "【合戦】{} が {} へ侵攻しました",
                attacker_name.0, defender_name.0
            ),
            DomesticLogEvent::WarAttackerOccupied {
                home_name,
                occupied_name,
            } => format!(
                "【合戦】{} が {} を占領しました",
                home_name.0, occupied_name.0
            ),
            DomesticLogEvent::WarDefenderDefended { defender_name } => {
                format!("【合戦】{} が侵攻を退けました", defender_name.0)
            }
        },
        ActionLogEvent::War(e) => match e {
            WarLogEvent::CpuDefenderTactic { tactic } => {
                format!("CPU Defender Tactic: {:?}", tactic)
            }
            WarLogEvent::Damage {
                attacker_tactic,
                defender_tactic,
                attacker_damage,
                defender_damage,
            } => {
                format!(
                    "自軍({})の被害: {}、敵軍({})の被害: {}",
                    attacker_tactic.name(),
                    attacker_damage,
                    defender_tactic.name(),
                    defender_damage
                )
            }
            WarLogEvent::AttackerVictory { home_name, .. } => format!(
                "合戦終了：攻撃軍（{}から出陣）の勝利！領地を占領しました",
                home_name.0
            ),
            WarLogEvent::DefenderVictory { .. } => "合戦終了：防衛軍の勝利".to_string(),
            WarLogEvent::WarStarted {
                attacker_name,
                defender_name,
                ..
            } => format!(
                "{} が {} へ侵攻を開始しました",
                attacker_name.0, defender_name.0
            ),
        },
    }
}
