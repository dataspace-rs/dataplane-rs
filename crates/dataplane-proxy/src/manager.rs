use async_trait::async_trait;
use edc_dataplane_core::{
    core::{
        model::transfer::{types::TransferKind, Transfer},
        service::transfer::TransferManager,
    },
    signaling::DataAddress,
};

use crate::{
    db::edr::EdrRepoRef,
    model::edr::EdrEntry,
    service::{edr::EdrManager, token::TokenManager},
};

pub struct TransferProxyManager<T: TokenManager> {
    edrs: EdrManager<T>,
    tokens: EdrRepoRef,
}

impl<T: TokenManager> TransferProxyManager<T> {
    pub fn new(edrs: EdrManager<T>, tokens: EdrRepoRef) -> Self {
        Self { edrs, tokens }
    }
}

#[async_trait]
impl<T: TokenManager + Send + Sync + 'static> TransferManager for TransferProxyManager<T> {
    async fn can_handle(&self, transfer: &Transfer) -> anyhow::Result<bool> {
        let _ = TransferKind::try_from(&transfer.source.0)?;

        Ok(true)
    }

    async fn handle_start(&self, transfer: &Transfer) -> anyhow::Result<Option<DataAddress>> {
        let edr = self.edrs.create_edr(transfer).await?;

        let entry = EdrEntry::builder()
            .transfer_id(transfer.id.clone())
            .refresh_token_id(edr.refresh_token_id)
            .token_id(edr.token_id)
            .build();

        self.tokens.save(entry).await?;

        Ok(Some(edr.data_address))
    }

    async fn handle_suspend(&self, _id: &str) -> anyhow::Result<()> {
        // todo handle suspend
        Ok(())
    }
    async fn handle_terminate(&self, id: &str) -> anyhow::Result<()> {
        self.edrs.delete(id).await
    }
}
