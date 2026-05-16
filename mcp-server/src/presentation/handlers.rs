use engine::application::usecase::battle_usecase::BattleUseCase;
use engine::application::usecase::daimyo_query_usecase::DaimyoQueryUseCase;
use engine::application::usecase::domestic_usecase::DomesticUseCase;
use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::kuni_query_usecase::KuniQueryUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use engine::domain::model::action_log::*;
use engine::domain::model::battle::Tactic;
use engine::domain::model::value_objects::*;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters, ServerHandler},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct McpHandlers {
    turn_progression_usecase: Arc<TurnProgressionUseCase>,
    domestic_usecase: Arc<DomesticUseCase>,
    battle_usecase: Arc<BattleUseCase>,
    kuni_query_usecase: Arc<KuniQueryUseCase>,
    info_usecase: Arc<InfoUseCase>,
    daimyo_query_usecase: Arc<DaimyoQueryUseCase>,
    selected_daimyo_id: Arc<Mutex<Option<DaimyoId>>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

// --- Parameter Structs ---

#[derive(Deserialize, JsonSchema)]
pub struct SelectDaimyoParams {
    /// 選択する大名のID
    pub daimyo_id: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct DomesticParams {
    /// 対象となる国のID
    pub kuni_id: u32,
    /// 実行する量（金、米、兵など）
    pub amount: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct TransportParams {
    /// 送り元の国ID
    pub from_kuni_id: u32,
    /// 送り先の国ID
    pub to_kuni_id: u32,
    /// 輸送する金の量
    pub kin: u32,
    /// 輸送する兵の数
    pub hei: u32,
    /// 輸送する米の量
    pub kome: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct StartWarParams {
    /// 攻撃側の国ID
    pub attacker_kuni_id: u32,
    /// 防御側の国ID
    pub defender_kuni_id: u32,
    /// 出陣させる兵の数
    pub hei: u32,
    /// 持参させる米の量
    pub kome: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExecuteBattleTurnParams {
    /// 攻撃側の国ID
    pub attacker_kuni_id: u32,
    /// 選択する戦術 (1: 通常, 2: 奇襲, 3: 火計, 4: 鼓舞, 5: 退却)
    pub tactic: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExecuteDefenseTurnParams {
    /// 防御側の国ID
    pub defender_kuni_id: u32,
    /// 選択する戦術 (1: 通常, 2: 奇襲, 3: 火計, 4: 鼓舞)
    pub tactic: u32,
}

#[derive(Deserialize, JsonSchema)]
pub struct AutoActionParams {
    /// 対象となる国のID
    pub kuni_id: u32,
}

impl McpHandlers {
    pub fn new(
        turn_progression_usecase: Arc<TurnProgressionUseCase>,
        domestic_usecase: Arc<DomesticUseCase>,
        battle_usecase: Arc<BattleUseCase>,
        kuni_query_usecase: Arc<KuniQueryUseCase>,
        info_usecase: Arc<InfoUseCase>,
        daimyo_query_usecase: Arc<DaimyoQueryUseCase>,
    ) -> Self {
        Self {
            turn_progression_usecase,
            domestic_usecase,
            battle_usecase,
            kuni_query_usecase,
            info_usecase,
            daimyo_query_usecase,
            selected_daimyo_id: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    async fn get_player_id(&self) -> Result<DaimyoId, String> {
        let lock = self.selected_daimyo_id.lock().await;
        lock.ok_or_else(|| {
            "大名が選択されていません。先に select_daimyo を実行してください。".to_string()
        })
    }

    async fn check_kuni_ownership(&self, kuni_id: KuniId) -> Result<DaimyoId, String> {
        let player_id = self.get_player_id().await?;

        // 選択された国の情報を探す
        let kunis = self
            .kuni_query_usecase
            .get_kunis_by_daimyo(&player_id)
            .await
            .map_err(|e| e.to_string())?;

        if !kunis.iter().any(|k| k.id == kuni_id) {
            return Err(format!(
                "国ID: {} はあなたの領地ではありません。",
                kuni_id.0
            ));
        }
        Ok(player_id)
    }

    fn parse_tactic(&self, tactic: u32, is_attacker: bool) -> Result<Tactic, String> {
        engine::domain::service::tactic_validation_service::TacticValidationService::parse_tactic(
            tactic,
            is_attacker,
        )
        .map_err(|e| e.to_string())
    }
}

#[tool_router(router = tool_router, vis = "pub")]
impl McpHandlers {
    /// 選択可能な大名の一覧を取得します
    #[tool(description = "選択可能な大名の一覧を取得します")]
    pub async fn list_daimyos(&self) -> Result<String, String> {
        let daimyos = self
            .daimyo_query_usecase
            .list()
            .await
            .map_err(|e| e.to_string())?;

        let mut result = String::from("選択可能な大名一覧:\n");
        for d in daimyos {
            result.push_str(&format!("- ID: {}, 名前: {}\n", d.id, d.name));
        }
        Ok(result)
    }

    /// 操作対象となる大名を選択します
    #[tool(description = "操作対象となる大名を選択します")]
    pub async fn select_daimyo(
        &self,
        Parameters(SelectDaimyoParams { daimyo_id }): Parameters<SelectDaimyoParams>,
    ) -> Result<String, String> {
        let id = DaimyoId::new(daimyo_id);
        let daimyo = self
            .daimyo_query_usecase
            .find(id)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(d) = daimyo {
            let mut lock = self.selected_daimyo_id.lock().await;
            *lock = Some(id);
            Ok(format!("大名「{}」を選択しました。", d.name))
        } else {
            Err(format!("ID: {} の大名が見つかりません。", daimyo_id))
        }
    }

    /// 現在の自分の状況（領地、資源、手番）を取得します
    #[tool(description = "現在の自分の状況（領地、資源、手番）を取得します")]
    pub async fn get_my_status(&self) -> Result<String, String> {
        let player_id = self.get_player_id().await?;
        let status = self
            .kuni_query_usecase
            .get_player_status(&player_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut result = format!("=== 第 {} ターン ===\n", status.current_turn);
        result.push_str(&format!("現在の手番: {}\n\n", status.current_daimyo_name));
        result.push_str("あなたの領地:\n");

        for k in &status.kunis {
            result.push_str(&format!(
                "- {} (ID: {}): 金={}, 米={}, 兵={}, 石高={}, 町={}, 忠誠={}\n",
                k.name,
                k.id.0,
                k.kin.value(),
                k.kome.value(),
                k.hei.value(),
                k.kokudaka.value(),
                k.machi.value(),
                k.tyu
            ));
        }

        if !status.defense_alerts.is_empty() {
            result.push_str("\n⚠️ 【緊急：侵攻検知】 ⚠️\n");
            for alert in status.defense_alerts {
                result.push_str(&format!(
                    "「{}」が「{}」に攻め込んでいます！\n",
                    alert.attacker_kuni_name, alert.defender_kuni_name
                ));
                result.push_str(&format!("敵軍勢: 兵数 {}\n", alert.enemy_hei.value()));
            }
            result.push_str("直ちに battle_execute_defense_turn で防衛戦術を指示してください。\n");
        }

        Ok(result)
    }

    /// 他国の情報を一覧で取得します
    #[tool(description = "他国の情報を一覧で取得します。実行にはコマンド実行権を1消費します。")]
    pub async fn get_other_countries_info(&self) -> Result<String, String> {
        let player_id = self.get_player_id().await?;
        let info = self
            .info_usecase
            .get_other_countries_info(Some(player_id), player_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut result = String::from("他国の情報一覧:\n");
        for c in info.countries {
            result.push_str(&format!(
                "- {} (領主: {}): 金={}, 米={}, 兵={}, 石高={}, 町={}, 忠誠={}\n",
                c.kuni_name,
                c.daimyo_name,
                c.kin.value(),
                c.kome.value(),
                c.hei.value(),
                c.kokudaka.value(),
                c.towns.value(),
                c.tyu
            ));
        }
        Ok(result)
    }

    /// 米を売却して金を得ます
    #[tool(description = "指定した国の米を売却して金を得ます")]
    pub async fn domestic_rice_sell(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        let gain = self
            .domestic_usecase
            .sell_rice(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("米を売却しました。得られた金: {}", gain.value()))
    }

    /// 金を払って米を購入します
    #[tool(description = "指定した国で金を払って米を購入します")]
    pub async fn domestic_rice_buy(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        let gain = self
            .domestic_usecase
            .buy_rice(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("米を購入しました。得られた米: {}", gain.value()))
    }

    /// 兵を徴募します
    #[tool(description = "指定した国で兵を徴募します。金が必要です。")]
    pub async fn domestic_recruit(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        self.domestic_usecase
            .recruit(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("兵を {} 人徴募しました。", amount))
    }

    /// 開墾を行い石高を上げます
    #[tool(description = "指定した国で開墾を行い石高を上げます。金が必要です。")]
    pub async fn domestic_develop_land(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        let gain = self
            .domestic_usecase
            .develop_land(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("開墾を行いました。上昇した石高: {}", gain.value()))
    }

    /// 町作りを行い、毎ターンの金収入を増やします
    #[tool(description = "指定した国で町作りを行い、毎ターンの金収入を増やします。")]
    pub async fn domestic_build_town(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        let gain = self
            .domestic_usecase
            .build_town(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("町作りを行いました。発展度: {}", gain.value()))
    }

    /// 民に施しを行い、忠誠度を上げます
    #[tool(description = "指定した国の民に施しを行い、忠誠度を上げます。")]
    pub async fn domestic_give_charity(
        &self,
        Parameters(DomesticParams { kuni_id, amount }): Parameters<DomesticParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;
        let gain = self
            .domestic_usecase
            .give_charity(Some(player_id), id, DisplayAmount::new(amount))
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("施しを行いました。上昇した忠誠度: {}", gain))
    }

    /// 隣接する自領の国へ資源を輸送します
    #[tool(description = "隣接する自領の国へ資源を輸送します。")]
    pub async fn domestic_transport(
        &self,
        Parameters(TransportParams {
            from_kuni_id,
            to_kuni_id,
            kin,
            hei,
            kome,
        }): Parameters<TransportParams>,
    ) -> Result<String, String> {
        let from_id = KuniId::new(from_kuni_id);
        let to_id = KuniId::new(to_kuni_id);
        let player_id = self.check_kuni_ownership(from_id).await?;

        // 輸送先も自分の領地かチェック
        let kunis = self
            .kuni_query_usecase
            .get_kunis_by_daimyo(&player_id)
            .await
            .map_err(|e| e.to_string())?;
        if !kunis.iter().any(|k| k.id == to_id) {
            return Err(format!(
                "輸送先の国ID: {} はあなたの領地ではありません。",
                to_id.0
            ));
        }

        self.domestic_usecase
            .transport(
                Some(player_id),
                from_id,
                to_id,
                DisplayAmount::new(kin),
                DisplayAmount::new(hei),
                DisplayAmount::new(kome),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok("資源を輸送しました。".to_string())
    }

    /// 隣接する他国へ合戦を仕掛けます
    #[tool(description = "隣接する他国へ合戦を仕掛けます。")]
    pub async fn battle_start_war(
        &self,
        Parameters(StartWarParams {
            attacker_kuni_id,
            defender_kuni_id,
            hei,
            kome,
        }): Parameters<StartWarParams>,
    ) -> Result<String, String> {
        let attacker_id = KuniId::new(attacker_kuni_id);
        let defender_id = KuniId::new(defender_kuni_id);
        let player_id = self.check_kuni_ownership(attacker_id).await?;

        let status = self
            .battle_usecase
            .start_war(
                Some(player_id),
                attacker_id,
                defender_id,
                DisplayAmount::new(hei),
                DisplayAmount::new(kome),
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!(
            "合戦を開始しました。攻撃側兵数: {}, 防御側兵数: {}",
            status.attacker.hei.to_display().value(),
            status.defender.hei.to_display().value()
        ))
    }

    /// 合戦のターンを1回進めます
    #[tool(
        description = "進行中の合戦のターンを1回進めます。戦術を選択してください (1: 通常, 2: 奇襲, 3: 火計, 4: 鼓舞, 5: 退却)"
    )]
    pub async fn battle_execute_turn(
        &self,
        Parameters(ExecuteBattleTurnParams {
            attacker_kuni_id,
            tactic,
        }): Parameters<ExecuteBattleTurnParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(attacker_kuni_id);
        let tactic_enum = self.parse_tactic(tactic, true)?;
        let player_id = self.check_kuni_ownership(id).await?;

        self.battle_usecase
            .execute_battle_turn(Some(player_id), id, tactic_enum)
            .await
            .map_err(|e| e.to_string())?;

        let next_status = self
            .battle_usecase
            .get_active_war(id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "合戦情報が見つかりません".to_string())?;

        let mut result = format!(
            "合戦ターン実行完了。残存兵数 - 攻: {}, 防: {}\n",
            next_status.attacker.hei.to_display().value(),
            next_status.defender.hei.to_display().value()
        );

        if let Some(winner) = next_status.winner {
            result.push_str(&format!("決着！ 勝者: {:?}", winner));
        }

        Ok(result)
    }

    /// 防御側として合戦のターンを1回進めます
    #[tool(
        description = "プレイヤーが防御側として、合戦のターンを1回進めます。戦術を選択してください (1: 通常, 2: 奇襲, 3: 火計, 4: 鼓舞)"
    )]
    pub async fn battle_execute_defense_turn(
        &self,
        Parameters(ExecuteDefenseTurnParams {
            defender_kuni_id,
            tactic,
        }): Parameters<ExecuteDefenseTurnParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(defender_kuni_id);
        let tactic_enum = self.parse_tactic(tactic, false)?;
        let player_id = self.check_kuni_ownership(id).await?;

        self.battle_usecase
            .execute_defense_turn(Some(player_id), id, tactic_enum)
            .await
            .map_err(|e| e.to_string())?;

        let next_status = self
            .battle_usecase
            .get_active_war(id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "合戦情報が見つかりません".to_string())?;

        let mut result = format!(
            "防衛ターン実行完了。残存兵数 - 攻: {}, 防: {}\n",
            next_status.attacker.hei.to_display().value(),
            next_status.defender.hei.to_display().value()
        );

        if let Some(winner) = next_status.winner {
            result.push_str(&format!("決着！ 勝者: {:?}", winner));
        }

        Ok(result)
    }

    /// 直近の行動ログを取得します
    #[tool(description = "直近の行動ログを取得します。")]
    pub async fn get_recent_logs(&self) -> Result<String, String> {
        let snapshot = self
            .kuni_query_usecase
            .get_ui_snapshot(None, None, None)
            .await
            .map_err(|e| e.to_string())?;

        let mut result = String::from("直近の行動ログ:\n");
        for log in snapshot.domestic_logs {
            result.push_str(&format!("- [ターン{}] {:?}\n", log.turn.value(), log.event));
        }
        Ok(result)
    }

    /// ゲームの進行処理（１ステップ）を実行します
    #[tool(
        description = "ゲームの進行処理を実行します。選択中の大名の手番になるか、1ターン終了するまで進みます。"
    )]
    pub async fn progress_turn(&self) -> Result<String, String> {
        let player_id = self.get_player_id().await?;

        self.turn_progression_usecase
            .progress_until_player_turn(Some(player_id))
            .await
            .map_err(|e| e.to_string())?;

        Ok("ゲームの進行処理を実行しました。".to_string())
    }

    /// 現在の手番の国に対して自動行動を実行します
    #[tool(
        description = "現在の手番の国に対して、AIが自動でコマンドを選択して実行します。実行後、手番が進みます。"
    )]
    pub async fn domestic_auto_action(
        &self,
        Parameters(AutoActionParams { kuni_id }): Parameters<AutoActionParams>,
    ) -> Result<String, String> {
        let id = KuniId::new(kuni_id);
        let player_id = self.check_kuni_ownership(id).await?;

        // 手番チェック
        let state = self
            .turn_progression_usecase
            .get_state()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "GameStateが見つかりません".to_string())?;

        state.check_turn(id).map_err(|e| e.to_string())?;

        // 自動行動の実行と手番進行 (原子的な実行)
        self.turn_progression_usecase
            .execute_cpu_action_and_advance(id, Some(player_id))
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("国ID: {} の自動行動を実行しました。", kuni_id))
    }

    /// デバッグ用の内部ログ（AIの思考プロセス含む）を取得します
    #[cfg(debug_assertions)]
    #[tool(description = "デバッグ用の内部ログ（AIの思考プロセス含む）を取得します。")]
    pub async fn get_internal_logs(&self) -> Result<String, String> {
        let logs = self
            .kuni_query_usecase
            .get_all_logs_internal(ActionLogCategory::Domestic)
            .map_err(|e| e.to_string())?;

        let mut result = String::from("内部ログ（デバッグ用）:\n");
        for log in logs {
            let visibility_str = match log.visibility {
                ActionLogVisibility::Public => "Public",
                ActionLogVisibility::Player => "Player",
                ActionLogVisibility::Internal => "Internal",
            };

            let event_str = match &log.event {
                ActionLogEvent::Domestic(DomesticLogEvent::CpuAction {
                    daimyo_id,
                    action_msg,
                    reasoning,
                }) => {
                    format!(
                        "AI行動 [大名ID:{:?}] {}. 理由: {}",
                        daimyo_id,
                        action_msg,
                        reasoning.as_deref().unwrap_or("なし")
                    )
                }
                _ => format!("{:?}", log.event),
            };

            result.push_str(&format!(
                "- [ターン{}] [{}] {}\n",
                log.turn.value(),
                visibility_str,
                event_str
            ));
        }
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for McpHandlers {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = Implementation::new("sengoku-mcp-server", env!("CARGO_PKG_VERSION"));
        info
    }
}
