use crate::application::dto::other_countries_info_dto::{CountryInfoDTO, OtherCountriesInfoDTO};
use crate::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use crate::domain::model::value_objects::DaimyoId;
use crate::domain::repository::{
    daimyo_repository::DaimyoRepository, game_state_repository::GameStateRepository,
    kuni_repository::KuniRepository,
};
use anyhow::Result;
use std::sync::Arc;

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
    pub async fn get_other_countries_info(
        &self,
        daimyo_id: DaimyoId,
    ) -> Result<OtherCountriesInfoDTO> {
        // 1. 手番の確認
        let state = self
            .game_state_repo
            .get()
            .await?
            .ok_or_else(|| anyhow::anyhow!("GameStateが見つかりません"))?;

        let current_kuni_id = state
            .current_kuni_id()
            .ok_or_else(|| anyhow::anyhow!("現在の手番の国が見つかりません"))?;

        state.check_turn(current_kuni_id)?;

        // 2. 全ての国情報を取得して集計
        let all_kunis = self.kuni_repo.find_all().await?;
        let mut country_infos = Vec::new();

        for kuni in all_kunis {
            // 自分（大名）の所有する国は含めない
            if kuni.daimyo_id == daimyo_id {
                continue;
            }

            let daimyo = self
                .daimyo_repo
                .find_by_id(&kuni.daimyo_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("大名が見つかりません: {:?}", kuni.daimyo_id))?;

            country_infos.push(CountryInfoDTO {
                kuni_id: kuni.id.0,
                kuni_name: kuni.name.0.clone(),
                daimyo_name: daimyo.name.0.clone(),
                kome: kuni.resource.kome.to_display(),
                kin: kuni.resource.kin.to_display(),
                hei: kuni.resource.hei.to_display(),
                kokudaka: kuni.stats.kokudaka.to_display(),
                towns: kuni.stats.machi.to_display(),
                tyu: kuni.stats.tyu.value(),
            });
        }

        // 3. アクションを完了させてターン（手番）を消費
        self.turn_progression_usecase
            .complete_current_action()
            .await?;

        Ok(OtherCountriesInfoDTO {
            countries: country_infos,
        })
    }
}
