use miwa::{
    core::{Extension, MiwaContext, MiwaResult},
    derive::extension,
};

use crate::core::{
    db::transfer::TransferRepoRef,
    service::transfer::{TransferManagerRef, TransferService},
};

pub struct TransferServiceExtension;

#[async_trait::async_trait]
impl Extension for TransferServiceExtension {
    async fn start(&self) -> MiwaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[extension(name = "Transfer Service sextension", provides(TransferService))]
pub async fn transfer_service_extension(
    ctx: &MiwaContext,
    manager: TransferManagerRef,
    repo: TransferRepoRef,
) -> MiwaResult<TransferServiceExtension> {
    ctx.register(TransferService::new(manager, repo));
    Ok(TransferServiceExtension)
}
