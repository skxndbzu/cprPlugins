use gateway_plugin_sdk::{
    ErrorCode, PluginFault,
    call::middleware::MiddlewareMount,
    client::{MiddlewareCall, MiddlewareResponse},
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub models: Vec<String>,
    pub account_group_ids: Vec<String>,
}

impl Config {
    pub fn from_value(value: Value) -> Result<Self, PluginFault> {
        let config: Self = serde_json::from_value(value).map_err(|_| {
            PluginFault::new(
                ErrorCode::InvalidInput,
                "Invalid Basis Points configuration",
            )
        })?;
        if config.models.len() > 128
            || config.models.iter().any(|model| {
                model.is_empty() || model.len() > 128 || model.chars().any(char::is_control)
            })
        {
            return Err(PluginFault::new(
                ErrorCode::InvalidInput,
                "Invalid model allowlist",
            ));
        }
        if config.account_group_ids.len() > 256
            || config.account_group_ids.iter().any(|id| {
                id.strip_prefix("grp_").is_none_or(|suffix| {
                    suffix.len() != 32
                        || !suffix
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                })
            })
        {
            return Err(PluginFault::new(
                ErrorCode::InvalidInput,
                "Invalid account group IDs",
            ));
        }
        Ok(config)
    }
}

#[derive(Clone)]
pub struct Adapter {
    config: Config,
}

impl Adapter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn handle(
        &self,
        mut call: MiddlewareCall,
    ) -> Result<MiddlewareResponse, PluginFault> {
        let head = &call.request.head;
        if head.mount != MiddlewareMount::Attempt
            || self.config.account_group_ids.is_empty()
            || head.provider.as_deref() != Some("openai")
            || head.protocol != "openai"
            || head.operation != "generate"
            || head.endpoint != "/v1/responses"
            || !head
                .model
                .as_ref()
                .is_some_and(|model| self.config.models.contains(model))
        {
            return call.next.run(call.request).await;
        }
        // 只选择宿主的协议适配，不读取凭据、不短路 Core 的调用和结算。
        // 未打补丁的宿主会在发送前拒绝这个协议，不能静默退回普通 Codex。
        let mut body: serde_json::Map<String, Value> = serde_json::from_slice(&call.request.body)
            .map_err(|_| {
            PluginFault::new(ErrorCode::InvalidInput, "Expected a visible JSON request")
        })?;
        // 宿主用选中账号的冻结分组事实判断，发送前移除此内部字段；不依据 Key 的允许组猜测。
        body.insert(
            "_cpr_basispoints_group_ids".into(),
            serde_json::json!(self.config.account_group_ids),
        );
        call.request.replace_body(
            serde_json::to_vec(&body)
                .map_err(|_| PluginFault::new(ErrorCode::InvalidInput, "Invalid request body"))?,
        );
        call.request.head.protocol = "openai-basispoints-v2".into();
        let mut response = call.next.run(call.request).await?;
        response.protocol = "openai".into();
        Ok(response)
    }
}
