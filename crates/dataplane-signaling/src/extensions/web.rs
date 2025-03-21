use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use edc_dataplane_core::core::service::transfer::TransferService;

use edc_dataplane_core::web::{self, ServerHandle};
use miwa::{
    core::{Extension, ExtensionConfig, MiwaContext, MiwaResult},
    derive::{extension, ExtensionConfig},
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::web::state::Context;

pub struct SignalingApiExtension {
    cfg: SignalingApiConfig,
    ctx: Context,
    handle: Arc<Mutex<Option<ServerHandle>>>,
}

impl SignalingApiExtension {
    pub fn new(cfg: SignalingApiConfig, ctx: Context) -> Self {
        SignalingApiExtension {
            cfg,
            ctx,
            handle: Arc::default(),
        }
    }
}

#[async_trait::async_trait]
impl Extension for SignalingApiExtension {
    async fn start(&self) -> MiwaResult<()> {
        let handle = web::start_server(
            self.cfg.bind,
            self.cfg.port,
            crate::web::signaling_app(),
            self.ctx.clone(),
            "Signaling API",
        )
        .await?;
        self.handle.lock().await.replace(handle);
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[derive(Deserialize, ExtensionConfig, Clone)]
#[config(prefix = "signaling")]
pub struct SignalingApiConfig {
    #[serde(default = "default_signaling_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
}

#[extension(name = "DataPlane Signaling API extension")]
pub async fn signaling_api_extension(
    _ctx: &MiwaContext,
    ExtensionConfig(cfg): ExtensionConfig<SignalingApiConfig>,
    transfer_service: TransferService,
) -> MiwaResult<SignalingApiExtension> {
    Ok(SignalingApiExtension::new(
        cfg,
        Context::new(transfer_service),
    ))
}

pub fn default_signaling_port() -> u16 {
    8787
}

pub fn default_bind() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}
