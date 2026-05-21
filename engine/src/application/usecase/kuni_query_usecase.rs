use crate::domain::model::action_log::{ActionLogCategory, ActionLogEntry};
use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{DaimyoId, KuniId};
use crate::domain::repository::action_log_repository::ActionLogRepository;
use crate::domain::repository::daimyo_repository::DaimyoRepository;
use crate::domain::repository::game_state_repository::GameStateRepository;
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use crate::domain::service::kuni_service::KuniService;
use std::collections::HashMap;
use std::sync::Arc;

/// UI表示に必要な情報を一括保持するスナップショット
#[derive(Debug, Clone, Default)]
pub struct UiSnapshot {
    pub all_daimyos: Vec<Daimyo>,
    pub current_turn: Option<u32>,
    pub season_name: String,
    pub current_kuni: Option<Kuni>,
    pub current_daimyo: Option<Daimyo>,
    pub attacker_kuni: Option<Kuni>,
    pub defender_kuni: Option<Kuni>,
    pub kuni_names: HashMap<KuniId, String>,
    pub domestic_logs: Vec<ActionLogEntry>,
    pub war_logs: Vec<ActionLogEntry>,
    pub active_battles: Vec<crate::domain::model::battle::WarStatus>,
    pub all_kunis: Vec<Kuni>,
    pub phase: crate::domain::model::game_state::GamePhase,
    pub winner: Option<DaimyoId>,
}

/// 国の情報照会に関するユースケース
pub struct KuniQueryUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    daimyo_repo: Arc<dyn DaimyoRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
    action_log_repo: Arc<dyn ActionLogRepository>,
    battle_repo: Arc<dyn crate::domain::repository::battle_repository::BattleRepository>,
}

impl KuniQueryUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
        action_log_repo: Arc<dyn ActionLogRepository>,
        battle_repo: Arc<dyn crate::domain::repository::battle_repository::BattleRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            neighbor_repo,
            action_log_repo,
            battle_repo,
        }
    }

    /// UI表示に必要な情報を一括取得します
    pub async fn get_ui_snapshot(
        &self,
        _selected_kuni_id: Option<KuniId>,
        attacker_id: Option<KuniId>,
        defender_id: Option<KuniId>,
    ) -> anyhow::Result<UiSnapshot> {
        // 基本情報の取得
        let all_daimyos = self.daimyo_repo.find_all().await?;
        let all_kunis = self.kuni_repo.find_all().await?;
        let kuni_names = all_kunis.iter().map(|k| (k.id, k.name.0.clone())).collect();

        let mut snapshot = UiSnapshot {
            all_daimyos,
            kuni_names,
            season_name: "不明".to_string(),
            domestic_logs: self
                .action_log_repo
                .find_visible(ActionLogCategory::Domestic, 100)?,
            war_logs: self
                .action_log_repo
                .find_visible(ActionLogCategory::War, 100)?,
            active_battles: self.battle_repo.find_all().await?,
            all_kunis,
            ..Default::default()
        };

        // 現在の手番情報を取得（これを優先する）
        if let Some(state) = self.game_state_repo.get().await? {
            snapshot.current_turn = Some(state.current_turn().value());
            let season_idx = state.current_turn().season();
            snapshot.season_name = match season_idx {
                0 => "春".to_string(),
                1 => "夏".to_string(),
                2 => "秋".to_string(),
                3 => "冬".to_string(),
                _ => "不明".to_string(),
            };
            snapshot.phase = state.phase();
            snapshot.winner = state.winner();
            if let Some(kuni_id) = state.current_kuni_id() {
                if let Some(kuni) = self.kuni_repo.find_by_id(&kuni_id).await? {
                    snapshot.current_kuni = Some(kuni.clone());
                    snapshot.current_daimyo = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await?;
                }
            }
        }

        // 攻撃・守備国の取得
        if let Some(id) = attacker_id {
            if let Some(kuni) = self.kuni_repo.find_by_id(&id).await? {
                snapshot.attacker_kuni = Some(kuni.clone());
                // アタッカーがいる場合、その大名を「現在の大名」としても表示したいケースがあるため
                if snapshot.current_daimyo.is_none() {
                    snapshot.current_daimyo = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await?;
                }
            }
        }

        if let Some(id) = defender_id {
            snapshot.defender_kuni = self.kuni_repo.find_by_id(&id).await?;
        }

        Ok(snapshot)
    }

    /// 指定した大名が支配する国の一覧を取得します
    pub async fn get_kunis_by_daimyo(&self, daimyo_id: &DaimyoId) -> anyhow::Result<Vec<Kuni>> {
        self.kuni_repo
            .find_by_daimyo_id(daimyo_id)
            .await
            .map_err(|e| e.into())
    }

    /// 指定した国の隣接国を取得します
    pub async fn get_neighbors(&self, kuni_id: &KuniId) -> anyhow::Result<Vec<Kuni>> {
        KuniService::get_neighbor_kunis(
            kuni_id,
            self.neighbor_repo.as_ref(),
            self.kuni_repo.as_ref(),
        )
        .await
        .map_err(|e| e.into())
    }

    /// 指定した国の隣接国のID一覧を取得します
    pub fn get_neighbor_ids(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.neighbor_repo.get_neighbors(kuni_id)
    }

    /// 攻撃可能または輸送可能な隣接国のID一覧を取得します
    pub async fn get_filtered_neighbor_ids(
        &self,
        kuni_id: &KuniId,
        is_attack: bool,
    ) -> anyhow::Result<Vec<KuniId>> {
        let current_kuni = self
            .kuni_repo
            .find_by_id(kuni_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません"))?;

        let neighbors = KuniService::get_neighbor_kunis(
            kuni_id,
            self.neighbor_repo.as_ref(),
            self.kuni_repo.as_ref(),
        )
        .await?;

        let filtered = neighbors
            .into_iter()
            .filter(|k| {
                if is_attack {
                    k.daimyo_id != current_kuni.daimyo_id
                } else {
                    k.daimyo_id == current_kuni.daimyo_id
                }
            })
            .map(|k| k.id)
            .collect();

        Ok(filtered)
    }
    /// 全ての行動ログ（内部ログを含む）を取得します (デバッグ用)
    pub fn get_all_logs_internal(
        &self,
        category: ActionLogCategory,
    ) -> anyhow::Result<Vec<ActionLogEntry>> {
        self.action_log_repo
            .find_all(category)
            .map_err(|e| e.into())
    }

    /// プレイヤーの現在の状況を取得します（防衛戦の警告含む）
    pub async fn get_player_status(
        &self,
        player_id: &DaimyoId,
    ) -> anyhow::Result<crate::application::dto::player_status_dto::PlayerStatusDTO> {
        let kunis = self.kuni_repo.find_by_daimyo_id(player_id).await?;
        let battles = self.battle_repo.find_all().await?;
        let all_kunis = self.kuni_repo.find_all().await?;
        let state = match self.game_state_repo.get().await? {
            Some(state) => state,
            None => {
                let order: Vec<KuniId> = all_kunis.iter().map(|k| k.id).collect();
                if order.is_empty() {
                    return Err(anyhow::anyhow!(
                        "国が存在しないため初期 GameState を生成できません"
                    ));
                }
                crate::domain::model::game_state::GameState::new(
                    crate::domain::model::value_objects::TurnNumber::new(1),
                    order,
                    crate::domain::model::value_objects::ActionOrderIndex::new(0),
                )?
            }
        };
        let kuni_names: HashMap<KuniId, String> =
            all_kunis.iter().map(|k| (k.id, k.name.0.clone())).collect();

        let current_turn = state.current_turn().value();
        let current_daimyo_name = if let Some(kuni_id) = state.current_kuni_id() {
            let kuni = self
                .kuni_repo
                .find_by_id(&kuni_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("手番の国が見つかりません"))?;
            let daimyo = self
                .daimyo_repo
                .find_by_id(&kuni.daimyo_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("手番の大名が見つかりません"))?;
            daimyo.name.0.clone()
        } else {
            "不明".to_string()
        };

        let my_kuni_ids: std::collections::HashSet<_> = kunis.iter().map(|k| k.id).collect();
        let mut defense_alerts = Vec::new();

        for battle in battles {
            if my_kuni_ids.contains(&battle.defender.kuni_id) {
                defense_alerts.push(
                    crate::application::dto::player_status_dto::DefenseAlertDTO {
                        attacker_kuni_name: kuni_names
                            .get(&battle.attacker.kuni_id)
                            .cloned()
                            .unwrap_or_else(|| "不明".to_string()),
                        defender_kuni_name: kuni_names
                            .get(&battle.defender.kuni_id)
                            .cloned()
                            .unwrap_or_else(|| "不明".to_string()),
                        enemy_hei: battle.attacker.hei.to_display(),
                    },
                );
            }
        }

        let kuni_dtos = kunis
            .into_iter()
            .map(
                |k| crate::application::dto::player_status_dto::KuniStatusDTO {
                    id: k.id,
                    name: k.name.0,
                    kin: k.resource.kin.to_display(),
                    kome: k.resource.kome.to_display(),
                    hei: k.resource.hei.to_display(),
                    jinko: k.resource.jinko.to_display(),
                    kokudaka: k.stats.kokudaka.to_display(),
                    machi: k.stats.machi.to_display(),
                    tyu: k.stats.tyu.value(),
                },
            )
            .collect();

        Ok(
            crate::application::dto::player_status_dto::PlayerStatusDTO {
                current_turn,
                current_daimyo_name,
                kunis: kuni_dtos,
                defense_alerts,
            },
        )
    }
}
