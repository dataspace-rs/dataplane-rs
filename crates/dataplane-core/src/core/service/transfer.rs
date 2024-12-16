use tracing::debug;

use crate::{
    core::{
        db::transfer::TransferStoreRef,
        model::transfer::{Transfer, TransferStatus},
    },
    signaling::{DataFlowResponseMessage, DataFlowStartMessage},
};

use super::{edr::EdrManager, token::TokenManager};

#[derive(Clone)]
pub struct TransferManager<T: TokenManager> {
    db: TransferStoreRef,
    pub(crate) edrs: EdrManager<T>,
}

impl<T: TokenManager> TransferManager<T> {
    pub fn new(edrs: EdrManager<T>, db: TransferStoreRef) -> Self {
        Self { db, edrs }
    }

    pub async fn start(
        &self,
        req: DataFlowStartMessage,
    ) -> anyhow::Result<DataFlowResponseMessage> {
        let edr = self.edrs.create_edr(&req).await?;

        let transfer = Transfer::builder()
            .id(req.process_id.clone())
            .source(req.source_data_address)
            .status(TransferStatus::Started)
            .refresh_token_id(edr.refresh_token_id)
            .token_id(edr.token_id)
            .build();

        self.db.save(transfer).await?;
        Ok(DataFlowResponseMessage::with_data_address(edr.data_address))
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Duration;
    use jsonwebtoken::errors::ErrorKind;
    use uuid::Uuid;

    use crate::{
        core::{
            db::transfer::{MockTransferStore, TransferStoreRef},
            model::{
                edr::EdrClaims,
                namespace::{EDC_NAMESPACE, IDSA_NAMESPACE},
            },
            service::{
                edr::EdrManager,
                token::{MockTokenManager, TokenError},
            },
        },
        signaling::{DataAddress, DataFlowStartMessage, FlowType},
    };

    use super::TransferManager;

    #[tokio::test]
    async fn start_transfer() {
        let mut token_manager = MockTokenManager::new();
        let mut store = MockTransferStore::new();

        token_manager
            .expect_issue::<EdrClaims>()
            .returning(|_: &EdrClaims| Ok("token".to_string()));

        store
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let manager = create_transfer_manager(token_manager, store);

        let req = create_req();

        let data_address = manager
            .start(req)
            .await
            .unwrap()
            .data_address
            .expect("Data address is missing");

        assert_eq!(data_address.endpoint_type, IDSA_NAMESPACE.to_iri("HTTP"));
        assert_eq!(data_address.endpoint_properties.len(), 7);

        assert_eq!(
            data_address.get_property(&EDC_NAMESPACE.to_iri("access_token")),
            Some("token")
        );
        assert_eq!(
            data_address.get_property(&EDC_NAMESPACE.to_iri("endpoint")),
            Some(manager.edrs.proxy_url.as_ref())
        );
    }

    #[tokio::test]
    async fn start_transfer_fails_when_store_fails() {
        let mut token_manager = MockTokenManager::new();
        let mut store = MockTransferStore::new();

        token_manager
            .expect_issue::<EdrClaims>()
            .returning(|_: &EdrClaims| Ok("token".to_string()));

        store
            .expect_save()
            .returning(|_| Box::pin(async { Err(anyhow::anyhow!("Failed to save")) }));

        let manager = create_transfer_manager(token_manager, store);

        let req = create_req();

        let result = manager.start(req).await.unwrap_err();

        assert_eq!(result.to_string(), "Failed to save");
    }

    #[tokio::test]
    async fn start_transfer_fails_when_token_creation_fails() {
        let mut token_manager = MockTokenManager::new();
        let store = MockTransferStore::new();

        token_manager
            .expect_issue::<EdrClaims>()
            .returning(|_: &EdrClaims| Err(TokenError::Encode(ErrorKind::InvalidAlgorithm.into())));

        let manager = create_transfer_manager(token_manager, store);

        let req = create_req();

        let result = manager.start(req).await.unwrap_err();

        assert_eq!(result.to_string(), "Error encoding token");
    }

    fn create_transfer_manager(
        mock: MockTokenManager,
        mock_store: MockTransferStore,
    ) -> TransferManager<MockTokenManager> {
        let edr = EdrManager::builder()
            .tokens(mock)
            .proxy_url("http://localhost:8080/public")
            .issuer("issuer")
            .token_duration(Duration::days(1))
            .token_url("http://localhost:8080/token")
            .jwks_url("http://localhost:8080/.well-known/jwks.json")
            .build();

        let store = TransferStoreRef::of(mock_store);

        TransferManager::new(edr, store)
    }

    fn create_req() -> DataFlowStartMessage {
        DataFlowStartMessage::builder()
            .participant_id("participant_id".to_string())
            .process_id("process_id".to_string())
            .source_data_address(
                DataAddress::builder()
                    .endpoint_type("MyType".to_string())
                    .endpoint_properties(vec![])
                    .build(),
            )
            .properties(HashMap::new())
            .flow_type(FlowType::Pull)
            .dataset_id(Uuid::new_v4().to_string())
            .agreement_id(Uuid::new_v4().to_string())
            .build()
    }
}
