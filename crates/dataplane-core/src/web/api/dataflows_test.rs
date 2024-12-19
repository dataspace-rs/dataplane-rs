mod start {
    use std::{collections::HashMap, sync::Arc};

    use axum::{body::Body, http::Request, Router};
    use chrono::Duration;
    use jsonwebtoken::{jwk::JwkSet, TokenData};
    use reqwest::{header::CONTENT_TYPE, StatusCode};
    use serde::{de::DeserializeOwned, Serialize};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{
        core::{
            db::transfer::{MockTransferStore, TransferStoreRef},
            service::{
                edr::EdrManager,
                refresh::RefreshManager,
                token::{MockTokenManager, TokenError, TokenManager},
                transfer::TransferManager,
            },
        },
        signaling::{DataAddress, DataFlowStartMessage, FlowType},
        web::{signaling_app, state::Context},
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

    fn create_context(
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

    #[tokio::test]
    async fn start() {
        let tokens = MockTokenManager::new();
        let store = MockTransferStore::new();
        let ctx = create_context(tokens, store);
        let app: Router = signaling_app::<MockTokenManagerWrapper>().with_state(ctx);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/dataflows")
                    .method("POST")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&create_req()).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
