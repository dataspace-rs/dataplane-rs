use std::sync::Arc;

use miwa::{
    core::{Extension, ExtensionConfig, MiwaResult},
    derive::extension,
};
use tokio::sync::Mutex;

use crate::{
    db::edr::EdrRepoRef,
    extensions::{
        config::Proxy,
        manager::{create_edr_manager, create_token_manager},
    },
    service::{refresh::RefreshManager, token::TokenManagerImpl},
    web::state::Context,
};
use edc_dataplane_core::{
    core::{db::transfer::TransferRepoRef, service::transfer::TransferService},
    web::{start_server, ServerHandle},
};

use crate::web;
pub struct DataPlaneProxyApiExtension {
    cfg: Proxy,
    ctx: Context<TokenManagerImpl>,
    handle: Arc<Mutex<Option<ServerHandle>>>,
}

#[async_trait::async_trait]
impl Extension for DataPlaneProxyApiExtension {
    async fn start(&self) -> MiwaResult<()> {
        let token_server = start_server(
            self.cfg.renewal.bind,
            self.cfg.renewal.port,
            web::token_app(),
            self.ctx.clone(),
            "Token renewal API",
        )
        .await?;

        self.handle.lock().await.replace(token_server);

        crate::web::proxy::server::start(&self.cfg, self.ctx.clone()).await;
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[extension(name = "Proxy api extension")]
pub async fn proxy_api_extension(
    ExtensionConfig(cfg): ExtensionConfig<Proxy>,
    repo: TransferRepoRef,
    edrs: EdrRepoRef,
    transfer_service: TransferService,
) -> MiwaResult<DataPlaneProxyApiExtension> {
    let tokens = create_token_manager(cfg.clone())?;
    let edr_manager = create_edr_manager(edrs, tokens.clone(), cfg.clone())?;

    let refresh_manager = RefreshManager::new(edr_manager, repo);
    let ctx = Context::new(transfer_service, tokens, refresh_manager);
    Ok(DataPlaneProxyApiExtension {
        cfg,
        ctx,
        handle: Arc::default(),
    })
}
