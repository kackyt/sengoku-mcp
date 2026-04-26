use crate::domain::model::daimyo::Daimyo;
use crate::domain::model::kuni::Kuni;
use crate::domain::model::value_objects::{DaimyoId, KuniId};
use crate::domain::repository::daimyo_repository::DaimyoRepository;
use crate::domain::repository::game_state_repository::GameStateRepository;
use crate::domain::repository::kuni_repository::KuniRepository;
use crate::domain::repository::neighbor_repository::NeighborRepository;
use std::collections::HashMap;
use std::sync::Arc;

/// UI表示に必要な情報を一括保持するスナップショット
#[derive(Debug, Clone, Default)]
pub struct UiSnapshot {
    pub all_daimyos: Vec<Daimyo>,
    pub current_turn: Option<u32>,
    pub current_kuni: Option<Kuni>,
    pub current_daimyo: Option<Daimyo>,
    pub attacker_kuni: Option<Kuni>,
    pub defender_kuni: Option<Kuni>,
    pub kuni_names: HashMap<KuniId, String>,
}

/// 国の情報照会に関するユースケース
pub struct KuniQueryUseCase {
    kuni_repo: Arc<dyn KuniRepository>,
    daimyo_repo: Arc<dyn DaimyoRepository>,
    game_state_repo: Arc<dyn GameStateRepository>,
    neighbor_repo: Arc<dyn NeighborRepository>,
}

impl KuniQueryUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository>,
        daimyo_repo: Arc<dyn DaimyoRepository>,
        game_state_repo: Arc<dyn GameStateRepository>,
        neighbor_repo: Arc<dyn NeighborRepository>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            neighbor_repo,
        }
    }

    /// UI表示に必要な情報を一括取得します
    pub async fn get_ui_snapshot(
        &self,
        selected_kuni_id: Option<KuniId>,
        attacker_id: Option<KuniId>,
        defender_id: Option<KuniId>,
    ) -> anyhow::Result<UiSnapshot> {
        // 基本情報の取得
        let all_daimyos = self.daimyo_repo.find_all().await?;
        let all_kunis = self.kuni_repo.find_all().await?;
        let kuni_names = all_kunis.into_iter().map(|k| (k.id, k.name.0)).collect();

        let current_turn = self
            .game_state_repo
            .get()
            .await?
            .map(|state| state.current_turn().value());

        let mut snapshot = UiSnapshot {
            all_daimyos,
            kuni_names,
            current_turn,
            ..Default::default()
        };

        // 特定の国の詳細情報を取得
        if let Some(id) = selected_kuni_id {
            if let Some(kuni) = self.kuni_repo.find_by_id(&id).await? {
                snapshot.current_kuni = Some(kuni.clone());
                snapshot.current_daimyo = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await?;
            }
        }

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

        // 現在の手番情報を取得
        if let Some(state) = self.game_state_repo.get().await? {
            snapshot.current_turn = Some(state.current_turn().value());
            if let Some(kuni_id) = state.current_kuni_id() {
                if let Some(kuni) = self.kuni_repo.find_by_id(&kuni_id).await? {
                    snapshot.current_kuni = Some(kuni.clone());
                    snapshot.current_daimyo = self.daimyo_repo.find_by_id(&kuni.daimyo_id).await?;
                }
            }
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
        let neighbor_ids = self.neighbor_repo.get_neighbors(kuni_id);
        let mut neighbors = Vec::with_capacity(neighbor_ids.len());
        for id in neighbor_ids {
            let kuni = self
                .kuni_repo
                .find_by_id(&id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("隣接国が見つかりません: {:?}", id))?;
            neighbors.push(kuni);
        }
        Ok(neighbors)
    }

    /// 指定した国の隣接国のID一覧を取得します
    pub fn get_neighbor_ids(&self, kuni_id: &KuniId) -> Vec<KuniId> {
        self.neighbor_repo.get_neighbors(kuni_id)
    }
}
