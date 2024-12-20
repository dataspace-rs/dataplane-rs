use std::{collections::HashMap, sync::Arc};

use chrono::Duration;
use jsonwebtoken::{jwk::JwkSet, TokenData};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::{
    core::{
        db::transfer::{MockTransferStore, TransferStoreRef},
        model::namespace::EDC_NAMESPACE,
        service::{
            edr::EdrManager,
            refresh::RefreshManager,
            token::{MockTokenManager, TokenError, TokenManager},
            transfer::TransferManager,
        },
    },
    signaling::{DataAddress, DataFlowStartMessage, EndpointProperty, FlowType},
    web::state::Context,
};

fn create_edr_manager(mock: MockTokenManagerWrapper) -> EdrManager<MockTokenManagerWrapper> {
    EdrManager::builder()
        .tokens(mock)
        .proxy_url("http://localhost:8080/public")
        .issuer("issuer")
        .token_duration(Duration::days(1))
        .token_url("http://localhost:8080/token")
        .jwks_url("http://localhost:8080/.well-known/jwks.json")
        .build()
}

fn create_transfer_manager(
    edrs: EdrManager<MockTokenManagerWrapper>,
    store: TransferStoreRef,
) -> TransferManager<MockTokenManagerWrapper> {
    TransferManager::new(edrs, store)
}

pub fn create_context(
    mock: MockTokenManager,
    store: MockTransferStore,
) -> Context<MockTokenManagerWrapper> {
    let store = TransferStoreRef::of(store);
    let wrapper = MockTokenManagerWrapper(Arc::new(mock));
    let edrs = create_edr_manager(wrapper.clone());
    let transfer_manager = create_transfer_manager(edrs.clone(), store.clone());
    let refresh_manager = RefreshManager::new(edrs.clone(), store.clone());

    Context::new(transfer_manager, wrapper, edrs, refresh_manager)
}

#[derive(Clone)]
pub struct MockTokenManagerWrapper(Arc<MockTokenManager>);

impl TokenManager for MockTokenManagerWrapper {
    fn issue<T: Serialize + 'static>(&self, claims: &T) -> Result<String, TokenError> {
        self.0.issue(claims)
    }

    fn validate<T: DeserializeOwned + 'static>(
        &self,
        token: &str,
    ) -> Result<TokenData<T>, TokenError> {
        self.0.validate(token)
    }

    fn keys(&self) -> Result<JwkSet, TokenError> {
        self.0.keys()
    }
}

pub fn create_start_request() -> DataFlowStartMessage {
    create_start_request_with_type("HttpData")
}

pub fn create_start_request_with_type(kind: &str) -> DataFlowStartMessage {
    DataFlowStartMessage::builder()
        .participant_id("participant_id".to_string())
        .process_id("process_id".to_string())
        .source_data_address(
            DataAddress::builder()
                .endpoint_type(kind)
                .endpoint_properties(vec![EndpointProperty::builder()
                    .name(EDC_NAMESPACE.to_iri("baseUrl"))
                    .value("http://localhost:8080/data".to_string())
                    .build()])
                .build(),
        )
        .properties(HashMap::new())
        .flow_type(FlowType::Pull)
        .dataset_id(Uuid::new_v4().to_string())
        .agreement_id(Uuid::new_v4().to_string())
        .build()
}
