use engine::application::usecase::info_usecase::InfoUseCase;
use engine::application::usecase::turn_progression_usecase::TurnProgressionUseCase;
use engine::domain::model::value_objects::DaimyoId;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters, ServerHandler},
    model::{ErrorCode, Implementation, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct McpHandlers {
    turn_progression_usecase: Arc<TurnProgressionUseCase>,
    info_usecase: Arc<InfoUseCase>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

/// 「他国の情報を一覧表示する」ツールのパラメータ
#[derive(Deserialize, JsonSchema)]
pub struct GetOtherCountriesInfoParams {
    /// 情報を表示する大名のID
    pub daimyo_id: u32,
}

impl McpHandlers {
    pub fn new(
        turn_progression_usecase: Arc<TurnProgressionUseCase>,
        info_usecase: Arc<InfoUseCase>,
    ) -> Self {
        Self {
            turn_progression_usecase,
            info_usecase,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router(router = tool_router, vis = "pub")]
impl McpHandlers {

    /// 「ターンを進める（自動行動を1ステップ進める）」ツールのハンドラ
    #[tool(description = "ゲームの進行処理（１ステップ）を実行します")]
    pub async fn progress_turn(&self) -> Result<String, rmcp::ErrorData> {
        self.turn_progression_usecase
            .progress()
            .await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        // 今回の仕様ではイベントとして状態を送信するため、戻り値は簡易な完了メッセージとする
        Ok(
            "ゲームの進行処理（１ステップ）を実行しました。イベントを確認してください。"
                .to_string(),
        )
    }


    /// 「他国の情報を一覧表示する」ツールのハンドラ
    #[tool(description = "指定した大名の視点で他国の情報を一覧表示します")]
    pub async fn get_other_countries_info(
        &self,
        Parameters(GetOtherCountriesInfoParams { daimyo_id }): Parameters<
            GetOtherCountriesInfoParams,
        >,
    ) -> Result<String, rmcp::ErrorData> {
        let info = self.info_usecase
            .get_other_countries_info(DaimyoId::new(daimyo_id))
            .await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        if info.countries.is_empty() {
            return Ok("他国の情報は見つかりませんでした。".to_string());
        }

        let mut result = String::from("他国の情報一覧:\n");
        for country in info.countries {
            result.push_str(&format!(
                "- {}: 米={}, 金={}, 兵={}, 石高={}, 町={}, 忠誠度(平均)={}\n",
                country.daimyo_name,
                country.kome,
                country.kin,
                country.hei,
                country.kokudaka,
                country.towns,
                country.tyu_avg
            ));
        }
        result.push_str("\n※このコマンドの実行により、手番を1つ消費しました。");

        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for McpHandlers {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("sengoku-mcp-server", env!("CARGO_PKG_VERSION"));
        info
    }
}
