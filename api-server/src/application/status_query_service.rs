use crate::application::dto::{DaimyoDto, KuniStatusDto, MyStatusDto, OtherKuniDto};
use engine::application::dto::player_status_dto::KuniStatusDTO;
use engine::domain::model::value_objects::{SessionId, ViewToken};
use engine::domain::repository::master_data_repository::MasterDataRepository;
use engine::domain::repository::session_repository::SessionRepository;
use engine::domain::repository::view_token_repository::ViewTokenRepository;
use game_session::game_lobby::PENDING_SESSION_PREFIX;
use game_session::GameContextFactory;
use std::sync::Arc;
use thiserror::Error;

/// 自国の状況照会で発生するエラー
#[derive(Debug, Error)]
pub enum StatusQueryError {
    /// 閲覧トークンが不正な形式、未発行、または失効済み
    #[error("閲覧URLが無効です（再発行された可能性があります）")]
    ViewTokenNotFound,
    /// 指定されたセッションがストレージに存在しない
    #[error("セッション '{0}' が見つかりません")]
    SessionNotFound(SessionId),
    /// セッションは存在するが大名が未選択
    #[error("大名がまだ選択されていません。チャットで大名を選択してください")]
    DaimyoNotSelected(SessionId),
    /// Webで作成したゲームに、まだチャット側が参加していない
    #[error("チャット側の参加待ちです。チャットで参加コードを伝えてください")]
    WaitingForJoin,
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
    view_tokens: Arc<dyn ViewTokenRepository>,
    master_data: Arc<dyn MasterDataRepository>,
}

impl StatusQueryService {
    /// 依存はドメイン層のリポジトリ trait で受け取る（具象実装は Composition Root で注入する）
    pub fn new(
        repository: Arc<dyn SessionRepository>,
        view_tokens: Arc<dyn ViewTokenRepository>,
        master_data: Arc<dyn MasterDataRepository>,
    ) -> Self {
        Self {
            repository,
            view_tokens,
            master_data,
        }
    }

    /// 閲覧トークンから対応するセッションの自国の状況を取得します
    ///
    /// WebアプリはセッションIDを知らなくても、ゲーム作成時に発行された
    /// URL（トークン）だけで状況を取得できます。
    pub async fn get_my_status_by_token(
        &self,
        token: &str,
    ) -> Result<MyStatusDto, StatusQueryError> {
        // 形式外のトークンは保存先へ問い合わせずに拒否する
        let token = ViewToken::parse(token).ok_or(StatusQueryError::ViewTokenNotFound)?;
        let session_id = self
            .view_tokens
            .find_session_id(&token)
            .await
            .map_err(anyhow::Error::from)?
            .ok_or(StatusQueryError::ViewTokenNotFound)?;

        // セッション期限切れ等で本体が消えている場合もトークン無効として扱い、セッションIDは返さない
        match self.get_my_status(&session_id).await {
            Err(StatusQueryError::SessionNotFound(_)) => Err(StatusQueryError::ViewTokenNotFound),
            // Webで作成した参加待ちのゲームは、大名未選択ではなく「参加待ち」として返す
            Err(StatusQueryError::DaimyoNotSelected(id))
                if id.value().starts_with(PENDING_SESSION_PREFIX) =>
            {
                Err(StatusQueryError::WaitingForJoin)
            }
            other => other,
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
        let territories = ctx.kuni_query_usecase.get_territories().await?;

        // 3. レスポンス用DTOへ変換する（他国は支配大名のみを公開し、資源は含めない）
        let other_kunis = territories
            .into_iter()
            .filter(|t| t.daimyo_id != player_id)
            .map(|t| OtherKuniDto {
                id: t.kuni_id.0,
                name: t.kuni_name,
                daimyo: DaimyoDto {
                    id: t.daimyo_id.0,
                    name: t.daimyo_name,
                },
            })
            .collect();

        Ok(MyStatusDto {
            turn: status.current_turn,
            daimyo: DaimyoDto {
                id: daimyo.id,
                name: daimyo.name,
            },
            my_kunis: status.kunis.iter().map(Self::to_kuni_dto).collect(),
            other_kunis,
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
}
