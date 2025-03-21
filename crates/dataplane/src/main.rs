use miwa::core::Miwa;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use edc_dataplane_core::extensions::{sql_repo_extension, transfer_service_extension};
use edc_dataplane_proxy::extensions::{
    proxy_api_extension, proxy_sql_repo_extension, transfer_proxy_extension,
};
use edc_dataplane_signaling::extensions::{registration_extension, signaling_api_extension};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(env_filter())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_file = std::env::var("DATAPLANE_CONFIG_FILE").ok();
    let mut handle = Miwa::prepare()
        .with_env("DP")
        .with_file(config_file)
        .build()?
        .add_extension(sql_repo_extension)
        .add_extension(proxy_sql_repo_extension)
        .add_extension(transfer_service_extension)
        .add_extension(transfer_proxy_extension)
        .add_extension(registration_extension)
        .add_extension(signaling_api_extension)
        .add_extension(proxy_api_extension)
        .start()
        .await?;

    info!("DataPlane started");
    handle.wait().await?;

    Ok(())
}

fn env_filter() -> EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into())
}
