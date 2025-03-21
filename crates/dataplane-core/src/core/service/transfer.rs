use async_trait::async_trait;
use miwa::derive::interface;

use miwa::derive::Injectable;
#[cfg(test)]
use mockall::{automock, predicate::*};
use tracing::debug;

use crate::{
    core::{
        db::transfer::TransferRepoRef,
        model::transfer::{Transfer, TransferStatus},
    },
    signaling::{DataAddress, DataFlowResponseMessage, DataFlowStartMessage},
};

#[derive(Clone, Injectable)]
pub struct TransferService {
    manager: TransferManagerRef,
    db: TransferRepoRef,
}

impl TransferService {
    pub fn new(manager: TransferManagerRef, db: TransferRepoRef) -> Self {
        Self { manager, db }
    }

    pub async fn start(
        &self,
        req: DataFlowStartMessage,
    ) -> anyhow::Result<DataFlowResponseMessage> {
        let transfer = Transfer::builder()
            .id(req.process_id.clone())
            .participant_id(req.participant_id.clone())
            .source(req.source_data_address)
            .status(TransferStatus::Started)
            .build();

        if self.manager.can_handle(&transfer).await? {
            let address = self.manager.handle_start(&transfer).await?;
            self.db.save(transfer).await?;
            Ok(DataFlowResponseMessage::new(address))
        } else {
            Err(anyhow::anyhow!("Transfer not supported"))
        }
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Transfer>> {
        self.db.fetch_by_id(id).await
    }

    pub async fn suspend(&self, id: String) -> anyhow::Result<()> {
        debug!("Suspending transfer with id {}", id);

        self.db.change_status(id, TransferStatus::Suspended).await
    }

    pub async fn terminate(&self, id: String, reason: Option<String>) -> anyhow::Result<()> {
        debug!(
            "Terminating transfer with id {} with reason: {:?}",
            id, reason
        );
        self.db.delete(&id).await
    }
}

#[async_trait]
#[interface]
#[cfg_attr(test, automock)]
pub trait TransferManager {
    async fn can_handle(&self, transfer: &Transfer) -> anyhow::Result<bool>;
    async fn handle_start(&self, transfer: &Transfer) -> anyhow::Result<Option<DataAddress>>;
    async fn handle_suspend(&self, id: &str) -> anyhow::Result<()>;
    async fn handle_terminate(&self, id: &str) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use std::collections::HashMap;
    use uuid::Uuid;

    use crate::{
        core::{
            db::transfer::{MockTransferRepo, TransferRepoRef},
            model::namespace::{EDC_NAMESPACE, IDSA_NAMESPACE},
        },
        signaling::{DataAddress, DataFlowStartMessage, EndpointProperty, FlowType},
    };

    use super::{MockTransferManager, TransferManagerRef, TransferService};

    #[tokio::test]
    async fn start_transfer() {
        // let mut token_manager = MockTokenManager::new();
        //
        let mut transfer_manager = MockTransferManager::new();
        let mut store = MockTransferRepo::new();

        transfer_manager
            .expect_can_handle()
            .returning(|_| futures::future::ok(true).boxed());

        transfer_manager
            .expect_handle_start()
            .returning(|_| futures::future::ok(Some(create_data_address())).boxed());

        store
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let manager = create_transfer_manager(transfer_manager, store);

        let req = create_req();

        let data_address = manager
            .start(req)
            .await
            .unwrap()
            .data_address
            .expect("Data address is missing");

        assert_eq!(data_address.endpoint_type, IDSA_NAMESPACE.to_iri("HTTP"));
        assert_eq!(data_address.endpoint_properties.len(), 0);
    }

    #[tokio::test]
    async fn start_transfer_fails_when_store_fails() {
        let mut transfer_manager = MockTransferManager::new();
        let mut store = MockTransferRepo::new();

        transfer_manager
            .expect_can_handle()
            .returning(|_| futures::future::ok(true).boxed());

        transfer_manager
            .expect_handle_start()
            .returning(|_| futures::future::ok(Some(create_data_address())).boxed());

        store
            .expect_save()
            .returning(|_| Box::pin(async { Err(anyhow::anyhow!("Failed to save")) }));

        let manager = create_transfer_manager(transfer_manager, store);

        let req = create_req();

        let result = manager.start(req).await.unwrap_err();

        assert_eq!(result.to_string(), "Failed to save");
    }

    #[tokio::test]
    async fn start_transfer_fails_when_manager_fails() {
        let mut transfer_manager = MockTransferManager::new();
        let store = MockTransferRepo::new();

        transfer_manager
            .expect_can_handle()
            .returning(|_| futures::future::ok(true).boxed());

        transfer_manager
            .expect_handle_start()
            .returning(|_| futures::future::err(anyhow::anyhow!("Failed to handle start")).boxed());

        let manager = create_transfer_manager(transfer_manager, store);

        let req = create_req();

        let result = manager.start(req).await.unwrap_err();

        assert_eq!(result.to_string(), "Failed to handle start");
    }

    fn create_transfer_manager(
        mock: MockTransferManager,
        mock_store: MockTransferRepo,
    ) -> TransferService {
        // let edr = EdrManager::builder()
        //     .tokens(mock)
        //     .proxy_url("http://localhost:8080/public")
        //     .issuer("issuer")
        //     .token_duration(Duration::days(1))
        //     .token_url("http://localhost:8080/token")
        //     .jwks_url("http://localhost:8080/.well-known/jwks.json")
        //     .build();

        let manager = TransferManagerRef::of(mock);
        let store = TransferRepoRef::of(mock_store);

        TransferService::new(manager, store)
    }

    fn create_data_address() -> DataAddress {
        DataAddress::builder()
            .endpoint_type(IDSA_NAMESPACE.to_iri("HTTP"))
            .endpoint_properties(vec![])
            .build()
    }

    fn create_req() -> DataFlowStartMessage {
        DataFlowStartMessage::builder()
            .participant_id("participant_id".to_string())
            .process_id("process_id".to_string())
            .source_data_address(
                DataAddress::builder()
                    .endpoint_type("HttpData".to_string())
                    .endpoint_properties(vec![EndpointProperty::builder()
                        .name(EDC_NAMESPACE.to_iri("baseUrl"))
                        .value("http://localhost:8080")
                        .build()])
                    .build(),
            )
            .properties(HashMap::new())
            .flow_type(FlowType::Pull)
            .dataset_id(Uuid::new_v4().to_string())
            .agreement_id(Uuid::new_v4().to_string())
            .build()
    }
}
