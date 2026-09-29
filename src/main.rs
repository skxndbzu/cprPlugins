use std::time::Duration;

use cpr_plugin_oai_basispoints::adapter::{Adapter, Config};
use gateway_plugin_sdk::client::{PluginBuilder, PluginSession, SessionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = PluginSession::accept(
        tokio::io::stdin(),
        tokio::io::stdout(),
        SessionConfig {
            maximum_call_timeout: Duration::from_secs(3600),
            ..SessionConfig::default()
        },
    )
    .await?;
    let config = Config::from_value(session.handshake().configuration.clone()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid Basis Points configuration",
        )
    })?;
    let adapter = Adapter::new(config);
    let plugin = PluginBuilder::from_json(include_bytes!("../plugin.json"))?
        .middleware(move |call| {
            let adapter = adapter.clone();
            async move { adapter.handle(call).await }
        })?
        .build()?;
    // stdout 专用于 SDK 帧；main 的错误只包含预定义分类，不输出配置或凭据。
    session.run(plugin).await?;
    Ok(())
}
