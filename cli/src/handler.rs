use crate::app::App;
use crate::screen::{DomesticCommand, DomesticSubState, ScreenState};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use engine::domain::model::kuni::Kuni;
use engine::domain::model::value_objects::{DisplayAmount, KuniId};

pub struct EventHandler;

impl EventHandler {
    pub async fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
        match &app.screen {
            ScreenState::Title => Self::handle_title(app, key).await,
            ScreenState::SelectDaimyo { cursor } => {
                Self::handle_select_daimyo(app, key, *cursor).await
            }
            ScreenState::Domestic {
                selected_kuni,
                cursor,
                sub_state,
            } => {
                let kuni_id = *selected_kuni;
                let cursor_pos = *cursor;
                let sub = sub_state.clone();
                Self::handle_domestic(app, key, kuni_id, cursor_pos, sub).await
            }
            ScreenState::War {
                status,
                cursor,
                sub_state,
            } => {
                let s = status.clone();
                let cur = *cursor;
                let sub = sub_state.clone();
                Self::handle_war(app, key, s, cur, sub).await
            }
            ScreenState::GameOver { .. } => {
                if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                    app.screen = ScreenState::Title;
                }
                Ok(())
            }
        }
    }

    async fn handle_title(app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                app.screen = ScreenState::SelectDaimyo { cursor: 0 };
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                app.running = false;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_select_daimyo(app: &mut App, key: KeyEvent, cursor: usize) -> Result<()> {
        let daimyos = &app.all_daimyos;
        if daimyos.is_empty() {
            app.screen = ScreenState::Title;
            return Ok(());
        }

        match key.code {
            KeyCode::Up => {
                app.screen = ScreenState::SelectDaimyo {
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Down if cursor < daimyos.len() - 1 => {
                app.screen = ScreenState::SelectDaimyo { cursor: cursor + 1 };
            }
            KeyCode::Enter => {
                let selected_daimyo = &daimyos[cursor];
                app.selected_daimyo_id = Some(selected_daimyo.id);

                app.turn_progression_usecase
                    .progress_until_player_turn(app.selected_daimyo_id)
                    .await?;

                let kunis: Vec<Kuni> = app
                    .kuni_query_usecase
                    .get_kunis_by_daimyo(&selected_daimyo.id)
                    .await?;
                if let Some(first_kuni) = kunis.first() {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: first_kuni.id,
                        cursor: 0,
                        sub_state: DomesticSubState::Normal,
                    };
                } else {
                    app.screen = ScreenState::Title;
                }
            }
            KeyCode::Esc => {
                app.screen = ScreenState::Title;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_domestic(
        app: &mut App,
        key: KeyEvent,
        kuni_id: KuniId,
        cursor: usize,
        sub_state: DomesticSubState,
    ) -> Result<()> {
        match sub_state {
            DomesticSubState::Normal => match key.code {
                KeyCode::Up => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor: cursor.saturating_sub(1),
                        sub_state: DomesticSubState::Normal,
                    };
                }
                KeyCode::Down if cursor < 12 => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor: cursor + 1,
                        sub_state: DomesticSubState::Normal,
                    };
                }
                KeyCode::Enter => {
                    let command = match cursor {
                        0 => DomesticCommand::War,
                        1 => DomesticCommand::SellRice,
                        2 => DomesticCommand::BuyRice,
                        3 => DomesticCommand::Develop,
                        4 => DomesticCommand::BuildTown,
                        5 => DomesticCommand::Hire,
                        6 => DomesticCommand::Dismiss,
                        7 => DomesticCommand::Give,
                        8 => DomesticCommand::Transport,
                        9 => DomesticCommand::Delegate,
                        10 => DomesticCommand::Undelegate,
                        11 => DomesticCommand::Information,
                        12 => DomesticCommand::Exit,
                        _ => return Ok(()),
                    };

                    if command != DomesticCommand::Exit
                        && !Self::check_player_turn(app, kuni_id, cursor).await?
                    {
                        return Ok(());
                    }

                    match command {
                        DomesticCommand::Exit => {
                            app.running = false;
                        }
                        DomesticCommand::Delegate | DomesticCommand::Undelegate => {
                            let delegate = command == DomesticCommand::Delegate;
                            app.domestic_usecase
                                .set_delegation(kuni_id, delegate)
                                .await?;
                            let msg = if delegate {
                                "委任しました"
                            } else {
                                "委任を解除しました"
                            };
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::ShowMessage {
                                    message: msg.to_string(),
                                    next_state: Box::new(DomesticSubState::Normal),
                                },
                            };
                        }
                        DomesticCommand::Information => {
                            app.turn_progression_usecase
                                .complete_current_action()
                                .await?;
                            app.turn_progression_usecase
                                .progress_until_player_turn(app.selected_daimyo_id)
                                .await?;
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::ShowMessage {
                                    message: "他国の情報を調査しました。各国の行動が進みました。"
                                        .to_string(),
                                    next_state: Box::new(DomesticSubState::Normal),
                                },
                            };
                        }
                        DomesticCommand::War => {
                            // 進行中の合戦があるか確認
                            if let Ok(Some(status)) =
                                app.battle_usecase.get_active_war(kuni_id).await
                            {
                                app.screen = ScreenState::War {
                                    status,
                                    cursor: 0,
                                    sub_state: crate::screen::WarSubState::Normal,
                                };
                            } else {
                                app.screen = ScreenState::Domestic {
                                    selected_kuni: kuni_id,
                                    cursor,
                                    sub_state: DomesticSubState::SelectTargetKuni {
                                        command,
                                        cursor: 0,
                                    },
                                };
                            }
                        }
                        DomesticCommand::Transport => {
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::SelectTargetKuni {
                                    command,
                                    cursor: 0,
                                },
                            };
                        }
                        _ => {
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::InputAmount {
                                    command,
                                    input: String::new(),
                                },
                            };
                        }
                    }
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::Title;
                }
                _ => {}
            },
            DomesticSubState::InputAmount { command, mut input } => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    input.push(c);
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputAmount { command, input },
                    };
                }
                KeyCode::Backspace => {
                    input.pop();
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputAmount { command, input },
                    };
                }
                KeyCode::Enter => {
                    let amount_val = match input.parse::<u32>() {
                        Ok(val) if val > 0 => val,
                        _ => {
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::ShowMessage {
                                    message: "1以上の数値を入力してください".to_string(),
                                    next_state: Box::new(DomesticSubState::InputAmount {
                                        command,
                                        input,
                                    }),
                                },
                            };
                            return Ok(());
                        }
                    };
                    let amount = DisplayAmount::new(amount_val);

                    let result = match command {
                        DomesticCommand::SellRice => app
                            .domestic_usecase
                            .sell_rice(kuni_id, amount)
                            .await
                            .map(|gain| format!("米を売却し、金を {} 獲得しました", gain)),
                        DomesticCommand::BuyRice => app
                            .domestic_usecase
                            .buy_rice(kuni_id, amount)
                            .await
                            .map(|cost| format!("米を購入し、金を {} 支払いました", cost)),
                        DomesticCommand::Develop => app
                            .domestic_usecase
                            .develop_land(kuni_id, amount)
                            .await
                            .map(|gain| format!("開墾完了！石高が {} 上昇しました", gain)),
                        DomesticCommand::BuildTown => app
                            .domestic_usecase
                            .build_town(kuni_id, amount)
                            .await
                            .map(|gain| format!("町造り完了！町ランクが {} 上昇しました", gain)),
                        DomesticCommand::Hire => app
                            .domestic_usecase
                            .recruit(kuni_id, amount)
                            .await
                            .map(|_| format!("兵を {} 雇用しました", amount_val)),
                        DomesticCommand::Dismiss => app
                            .domestic_usecase
                            .dismiss(kuni_id, amount)
                            .await
                            .map(|_| format!("兵を {} 解雇しました", amount_val)),
                        DomesticCommand::Give => app
                            .domestic_usecase
                            .give_charity(kuni_id, amount)
                            .await
                            .map(|gain| format!("施しを行い、忠誠度が {} 上昇しました", gain)),
                        _ => Ok("実行しました".to_string()),
                    };

                    match result {
                        Ok(result_msg) => {
                            app.turn_progression_usecase
                                .complete_current_action()
                                .await?;
                            app.turn_progression_usecase
                                .progress_until_player_turn(app.selected_daimyo_id)
                                .await?;

                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::ShowMessage {
                                    message: result_msg,
                                    next_state: Box::new(DomesticSubState::Normal),
                                },
                            };
                        }
                        Err(e) => {
                            app.screen = ScreenState::Domestic {
                                selected_kuni: kuni_id,
                                cursor,
                                sub_state: DomesticSubState::ShowMessage {
                                    message: format!("エラー: {}", e),
                                    next_state: Box::new(DomesticSubState::Normal),
                                },
                            };
                        }
                    }
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::Normal,
                    };
                }
                _ => {}
            },
            DomesticSubState::SelectTargetKuni {
                command,
                cursor: target_cursor,
            } => {
                let neighbors = app.kuni_query_usecase.get_neighbor_ids(&kuni_id);
                match key.code {
                    KeyCode::Up => {
                        app.screen = ScreenState::Domestic {
                            selected_kuni: kuni_id,
                            cursor,
                            sub_state: DomesticSubState::SelectTargetKuni {
                                command,
                                cursor: target_cursor.saturating_sub(1),
                            },
                        };
                    }
                    KeyCode::Down
                        if !neighbors.is_empty() && target_cursor < neighbors.len() - 1 =>
                    {
                        app.screen = ScreenState::Domestic {
                            selected_kuni: kuni_id,
                            cursor,
                            sub_state: DomesticSubState::SelectTargetKuni {
                                command,
                                cursor: target_cursor + 1,
                            },
                        };
                    }
                    KeyCode::Enter => {
                        if let Some(target_id) = neighbors.get(target_cursor) {
                            if command == DomesticCommand::War {
                                app.screen = ScreenState::Domestic {
                                    selected_kuni: kuni_id,
                                    cursor,
                                    sub_state: DomesticSubState::InputWarHeihe {
                                        target_id: *target_id,
                                        input: String::new(),
                                    },
                                };
                            } else if command == DomesticCommand::Transport {
                                let Some(kuni) = app.current_kuni.as_ref() else {
                                    app.screen = ScreenState::Domestic {
                                        selected_kuni: kuni_id,
                                        cursor,
                                        sub_state: DomesticSubState::ShowMessage {
                                            message: "国の情報が取得できませんでした".to_string(),
                                            next_state: Box::new(DomesticSubState::Normal),
                                        },
                                    };
                                    return Ok(());
                                };
                                let target_kin = kuni.resource.kin.mul_percent(10);
                                let target_hei = kuni.resource.hei.mul_percent(10);
                                let target_kome = kuni.resource.kome.mul_percent(10);

                                let result = app
                                    .domestic_usecase
                                    .transport(
                                        kuni_id,
                                        *target_id,
                                        target_kin.to_display(),
                                        target_hei.to_display(),
                                        target_kome.to_display(),
                                    )
                                    .await;

                                match result {
                                    Ok(_) => {
                                        app.turn_progression_usecase
                                            .complete_current_action()
                                            .await?;
                                        app.turn_progression_usecase
                                            .progress_until_player_turn(app.selected_daimyo_id)
                                            .await?;
                                        app.screen = ScreenState::Domestic {
                                            selected_kuni: kuni_id,
                                            cursor,
                                            sub_state: DomesticSubState::ShowMessage {
                                                message: "資源を輸送しました".to_string(),
                                                next_state: Box::new(DomesticSubState::Normal),
                                            },
                                        };
                                    }
                                    Err(e) => {
                                        app.screen = ScreenState::Domestic {
                                            selected_kuni: kuni_id,
                                            cursor,
                                            sub_state: DomesticSubState::ShowMessage {
                                                message: format!("エラー: {}", e),
                                                next_state: Box::new(DomesticSubState::Normal),
                                            },
                                        };
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.screen = ScreenState::Domestic {
                            selected_kuni: kuni_id,
                            cursor,
                            sub_state: DomesticSubState::Normal,
                        };
                    }
                    _ => {}
                }
            }
            DomesticSubState::InputWarHeihe {
                target_id,
                mut input,
            } => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    input.push(c);
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputWarHeihe { target_id, input },
                    };
                }
                KeyCode::Backspace => {
                    input.pop();
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputWarHeihe { target_id, input },
                    };
                }
                KeyCode::Enter => {
                    let amount = input.parse::<u32>().unwrap_or(0);
                    if amount > 0 {
                        app.screen = ScreenState::Domestic {
                            selected_kuni: kuni_id,
                            cursor,
                            sub_state: DomesticSubState::InputWarKome {
                                target_id,
                                hei: amount,
                                input: String::new(),
                            },
                        };
                    }
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::Normal,
                    };
                }
                _ => {}
            },
            DomesticSubState::InputWarKome {
                target_id,
                hei,
                mut input,
            } => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    input.push(c);
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputWarKome {
                            target_id,
                            hei,
                            input,
                        },
                    };
                }
                KeyCode::Backspace => {
                    input.pop();
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputWarKome {
                            target_id,
                            hei,
                            input,
                        },
                    };
                }
                KeyCode::Enter => {
                    let kome = input.parse::<u32>().unwrap_or(0);
                    if kome > 0 {
                        let start_result = app
                            .battle_usecase
                            .start_war(
                                kuni_id,
                                target_id,
                                DisplayAmount::new(hei),
                                DisplayAmount::new(kome),
                            )
                            .await;

                        match start_result {
                            Ok(new_status) => {
                                app.screen = ScreenState::War {
                                    status: new_status,
                                    cursor: 0,
                                    sub_state: crate::screen::WarSubState::Normal,
                                };
                            }
                            Err(e) => {
                                app.screen = ScreenState::Domestic {
                                    selected_kuni: kuni_id,
                                    cursor,
                                    sub_state: DomesticSubState::ShowMessage {
                                        message: format!("エラー: {}", e),
                                        next_state: Box::new(DomesticSubState::Normal),
                                    },
                                };
                            }
                        }
                    }
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: DomesticSubState::InputWarHeihe {
                            target_id,
                            input: hei.to_string(),
                        },
                    };
                }
                _ => {}
            },
            DomesticSubState::ShowMessage { next_state, .. } => {
                if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: kuni_id,
                        cursor,
                        sub_state: *next_state,
                    };
                }
            }
        }
        Ok(())
    }

    async fn handle_war(
        app: &mut App,
        key: KeyEvent,
        status: engine::domain::model::battle::WarStatus,
        cursor: usize,
        sub_state: crate::screen::WarSubState,
    ) -> Result<()> {
        use crate::screen::WarSubState;

        match sub_state {
            WarSubState::Normal => match key.code {
                KeyCode::Enter => {
                    app.screen = ScreenState::War {
                        status,
                        cursor: 0,
                        sub_state: WarSubState::SelectTactic,
                    };
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::Domestic {
                        selected_kuni: status.attacker_id,
                        cursor: 0,
                        sub_state: DomesticSubState::Normal,
                    };
                }
                _ => {}
            },
            WarSubState::SelectTactic => match key.code {
                KeyCode::Up => {
                    app.screen = ScreenState::War {
                        status,
                        cursor: cursor.saturating_sub(1),
                        sub_state: WarSubState::SelectTactic,
                    };
                }
                KeyCode::Down if cursor < 4 => {
                    app.screen = ScreenState::War {
                        status,
                        cursor: cursor + 1,
                        sub_state: WarSubState::SelectTactic,
                    };
                }
                KeyCode::Enter => {
                    use engine::domain::model::battle::Tactic;
                    let tactic = match cursor {
                        0 => Tactic::Normal,
                        1 => Tactic::Surprise,
                        2 => Tactic::Fire,
                        3 => Tactic::Inspire,
                        4 => Tactic::Retreat,
                        _ => Tactic::Normal,
                    };

                    if !Self::check_player_turn(app, status.attacker_id, 0).await? {
                        return Ok(());
                    }

                    let result_status = app
                        .battle_usecase
                        .execute_battle_turn(status.clone(), tactic)
                        .await?;

                    let msg = if let Some(winner) = result_status.winner {
                        use engine::domain::model::battle::BattleSide;
                        match winner {
                            BattleSide::Attacker => "我が軍の勝利！敵国を占領したぞ！".to_string(),
                            BattleSide::Defender => "我が軍の敗北...。兵力と兵糧を失ってしまった。".to_string(),
                        }
                    } else {
                        use engine::domain::model::battle::BattleAdvantage;
                        match result_status.advantage {
                            BattleAdvantage::Advantage => "我が軍が圧倒している！".to_string(),
                            BattleAdvantage::Even => "一進一退の攻防が続いている...！".to_string(),
                            BattleAdvantage::Disadvantage => "我が軍は苦戦を強いられている...！".to_string(),
                        }
                    };

                    if result_status.winner.is_some() {
                        app.turn_progression_usecase
                            .complete_current_action()
                            .await?;
                        app.turn_progression_usecase
                            .progress_until_player_turn(app.selected_daimyo_id)
                            .await?;
                        app.screen = ScreenState::Domestic {
                            selected_kuni: status.attacker_id,
                            cursor: 0,
                            sub_state: DomesticSubState::ShowMessage {
                                message: msg,
                                next_state: Box::new(DomesticSubState::Normal),
                            },
                        };
                    } else {
                        app.screen = ScreenState::War {
                            status: result_status,
                            cursor: 0,
                            sub_state: WarSubState::ShowMessage {
                                message: msg,
                                next_state: Box::new(WarSubState::Normal),
                            },
                        };
                    }
                }
                KeyCode::Esc => {
                    app.screen = ScreenState::War {
                        status,
                        cursor: 0,
                        sub_state: WarSubState::Normal,
                    };
                }
                _ => {}
            },
            WarSubState::ShowMessage { next_state, .. } => {
                if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                    app.screen = ScreenState::War {
                        status,
                        cursor: 0,
                        sub_state: *next_state,
                    };
                }
            }
        }
        Ok(())
    }

    async fn check_player_turn(app: &mut App, kuni_id: KuniId, cursor: usize) -> Result<bool> {
        let snapshot = app
            .kuni_query_usecase
            .get_ui_snapshot(None, None, None)
            .await?;
        if let Some(player_id) = app.selected_daimyo_id
            && let Some(current_daimyo) = snapshot.current_daimyo
            && current_daimyo.id == player_id
        {
            return Ok(true);
        }

        app.screen = ScreenState::Domestic {
            selected_kuni: kuni_id,
            cursor,
            sub_state: DomesticSubState::ShowMessage {
                message: "自分の手番ではありません".to_string(),
                next_state: Box::new(DomesticSubState::Normal),
            },
        };
        Ok(false)
    }
}
