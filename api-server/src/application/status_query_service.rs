use crate::application::dto::{
    DaimyoDto, DefenseAlertDto, GameProgressDto, KuniStatusDto, MyStatusDto, ResourceTotalsDto,
};
use engine::application::dto::player_status_dto::KuniStatusDTO;
use engine::domain::model::value_objects::{DisplayAmount, SessionId};
use engine::domain::repository::session_repository::SessionRepository;
use game_session::GameContextFactory;
use infrastructure::master_data::MasterDataLoader;
use std::sync::Arc;
use thiserror::Error;

/// 自国の状況照会で発生するエラー
#[derive(Debug, Error)]
pub enum StatusQueryError {
    /// 指定されたセッションがストレージに存在しない
    #[error("セッション '{0}' が見つかりません")]
    SessionNotFound(SessionId),
    /// セッションは存在するが大名が未選択
    #[error("セッション '{0}' では大名が選択されていません")]
    DaimyoNotSelected(SessionId),
    /// ストレージ障害やデータ不整合など
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// 共有ストレージからセッションを読み込み、自国の状況を組み立てる読み取り専用サービス
///
/// MCPサーバーは操作のたびにセッションを `SessionRepository` へ保存するため、
/// 本サービスはリクエストごとにストレージから最新状態を読み込みます（キャッシュしない）。
/// セッションの新規作成や保存は行いません。
pub struct StatusQueryService {
    repository: Arc<dyn SessionRepository>,
    master_data: Arc<MasterDataLoader>,
}

impl StatusQueryService {
    pub fn new(repository: Arc<dyn SessionRepository>, master_data: Arc<MasterDataLoader>) -> Self {
        Self {
            repository,
            master_data,
        }
    }

    /// 指定セッションにおける自国の状況を取得します
    pub async fn get_my_status(
        &self,
        session_id: &SessionId,
    ) -> Result<MyStatusDto, StatusQueryError> {
        // 1. 共有ストレージから最新のセッションデータを読み込む
        let data = self
            .repository
            .load(session_id)
            .await
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| StatusQueryError::SessionNotFound(session_id.clone()))?;

        let player_id = data
            .selected_daimyo_id
            .ok_or_else(|| StatusQueryError::DaimyoNotSelected(session_id.clone()))?;
        let last_accessed_at = data.last_accessed_at;

        // 2. MCPサーバーと同じ手順でゲームコンテキストを復元し、既存ユースケースで集計する
        let ctx = GameContextFactory::create_from_data(data, self.master_data.clone()).await?;

        let daimyo = ctx
            .daimyo_query_usecase
            .find(player_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("選択中の大名 (ID: {}) が見つかりません", player_id.0)
            })?;
        let status = ctx.kuni_query_usecase.get_player_status(&player_id).await?;
        let snapshot = ctx
            .kuni_query_usecase
            .get_ui_snapshot(None, None, None)
            .await?;

        // 3. レスポンス用DTOへ変換する
        let winner = snapshot
            .winner
            .and_then(|id| snapshot.all_daimyos.iter().find(|d| d.id == id))
            .map(|d| d.name.0.clone());

        Ok(MyStatusDto {
            session_id: session_id.value().to_string(),
            last_accessed_at,
            daimyo: DaimyoDto {
                id: daimyo.id,
                name: daimyo.name,
            },
            game: GameProgressDto {
                turn: status.current_turn,
                season: snapshot.season_name,
                phase: format!("{:?}", snapshot.phase),
                current_daimyo_name: status.current_daimyo_name,
                winner,
            },
            totals: Self::sum_resources(&status.kunis),
            kunis: status.kunis.iter().map(Self::to_kuni_dto).collect(),
            defense_alerts: status
                .defense_alerts
                .into_iter()
                .map(|alert| DefenseAlertDto {
                    attacker_kuni_name: alert.attacker_kuni_name,
                    defender_kuni_name: alert.defender_kuni_name,
                    enemy_hei: alert.enemy_hei.value(),
                })
                .collect(),
        })
    }

    /// 国の状況DTOをレスポンス形式に変換します
    fn to_kuni_dto(k: &KuniStatusDTO) -> KuniStatusDto {
        KuniStatusDto {
            id: k.id.0,
            name: k.name.clone(),
            kin: k.kin.value(),
            kome: k.kome.value(),
            hei: k.hei.value(),
            jinko: k.jinko.value(),
            kokudaka: k.kokudaka.value(),
            machi: k.machi.value(),
            tyu: k.tyu,
        }
    }

    /// 自領全体の資源を合計します（表示値同士の飽和加算）
    fn sum_resources(kunis: &[KuniStatusDTO]) -> ResourceTotalsDto {
        let zero = DisplayAmount::zero();
        let (kin, kome, hei, jinko, kokudaka) = kunis.iter().fold(
            (zero, zero, zero, zero, zero),
            |(kin, kome, hei, jinko, kokudaka), k| {
                (
                    kin.add(k.kin),
                    kome.add(k.kome),
                    hei.add(k.hei),
                    jinko.add(k.jinko),
                    kokudaka.add(k.kokudaka),
                )
            },
        );
        ResourceTotalsDto {
            kuni_count: kunis.len(),
            kin: kin.value(),
            kome: kome.value(),
            hei: hei.value(),
            jinko: jinko.value(),
            kokudaka: kokudaka.value(),
        }
    }
}
