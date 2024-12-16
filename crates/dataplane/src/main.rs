use dataplane_core::DataPlane;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(env_filter())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_file = std::env::var("DATAPLANE_CONFIG_FILE").ok();
    let mut handle = DataPlane::builder()
        .with_config_file(config_file)
        .prepare()?
        .start()
        .await?;

    handle.wait().await?;

    Ok(())
}

fn env_filter() -> EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into())
}
