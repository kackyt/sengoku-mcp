use crate::application::dto::other_countries_info_dto::{CountryInfoDTO, OtherCountriesInfoDTO};
use crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use crate::domain::model::value_objects::{DaimyoId, DisplayAmount};
use crate::domain::repository::{
    daimyo_repository::DaimyoRepository, game_state_repository::GameStateRepository,
    kuni_repository::KuniRepository,
};
use std::sync::Arc;
use anyhow::Result;

/// 情報コマンドに関するユースケース
pub struct InfoUseCase {
    kuni_repo: Arc<dyn KuniRepository + Send + Sync>,
    daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
    game_state_repo: Arc<dyn GameStateRepository + Send + Sync>,
    turn_progression_usecase: Arc<TurnProgressionUseCase>,
}

impl InfoUseCase {
    pub fn new(
        kuni_repo: Arc<dyn KuniRepository + Send + Sync>,
        daimyo_repo: Arc<dyn DaimyoRepository + Send + Sync>,
        game_state_repo: Arc<dyn GameStateRepository + Send + Sync>,
        turn_progression_usecase: Arc<TurnProgressionUseCase>,
    ) -> Self {
        Self {
            kuni_repo,
            daimyo_repo,
            game_state_repo,
            turn_progression_usecase,
        }
    }

    /// 自分以外の他国の情報を一覧で取得します。
    /// 実行には1アクション（1ターン）を消費します。
    pub async fn get_other_countries_info(&self, daimyo_id: DaimyoId) -> Result<OtherCountriesInfoDTO> {
        // 1. 手番の確認
        let state = self.game_state_repo.get().await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;
        
        let current_kuni_id = state.current_kuni_id()
            .ok_or_else(|| anyhow::anyhow!("現在の手番の国が見つかりません"))?;
        
        let current_kuni = self.kuni_repo.find_by_id(&current_kuni_id).await?
            .ok_or_else(|| anyhow::anyhow!("国が見つかりません: {:?}", current_kuni_id))?;
        
        if current_kuni.daimyo_id != daimyo_id {
            return Err(anyhow::anyhow!("あなたの手番ではありません。現在の手番: {:?}", current_kuni.daimyo_id));
        }

        // 2. 全ての大名と領地情報を取得して集計
        let all_daimyos = self.daimyo_repo.find_all().await?;
        let mut country_infos = Vec::new();

        for daimyo in all_daimyos {
            // 自分の情報は含めない
            if daimyo.id == daimyo_id {
                continue;
            }

            let kunis = self.kuni_repo.find_by_daimyo_id(&daimyo.id).await?;
            if kunis.is_empty() {
                continue;
            }

            let mut kome_total = 0;
            let mut kin_total = 0;
            let mut hei_total = 0;
            let mut kokudaka_total = 0;
            let mut machi_total = 0;
            let mut tyu_sum = 0;

            for kuni in &kunis {
                kome_total += kuni.resource.kome.to_display().value();
                kin_total += kuni.resource.kin.to_display().value();
                hei_total += kuni.resource.hei.to_display().value();
                kokudaka_total += kuni.stats.kokudaka.to_display().value();
                machi_total += kuni.stats.machi.to_display().value();
                tyu_sum += kuni.stats.tyu.value();
            }

            country_infos.push(CountryInfoDTO {
                daimyo_id: daimyo.id.value(),
                daimyo_name: daimyo.name.0.clone(),
                kome: DisplayAmount::new(kome_total),
                kin: DisplayAmount::new(kin_total),
                hei: DisplayAmount::new(hei_total),
                kokudaka: DisplayAmount::new(kokudaka_total),
                towns: DisplayAmount::new(machi_total),
                tyu_avg: tyu_sum / kunis.len() as u32,
            });
        }

        // 3. アクションを完了させてターン（手番）を消費
        self.turn_progression_usecase.complete_current_action().await?;

        Ok(OtherCountriesInfoDTO {
            countries: country_infos,
        })
    }
}
