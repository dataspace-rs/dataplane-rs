mod start {
    use axum::{body::Body, http::Request, Router};
    use http_body_util::BodyExt;
    use reqwest::{header::CONTENT_TYPE, StatusCode};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{
        core::{
            db::transfer::MockTransferStore,
            model::{edr::EdrClaims, namespace::EDC_NAMESPACE},
            service::token::MockTokenManager,
        },
        signaling::DataFlowResponseMessage,
        web::{
            api::fixtures::{
                create_context, create_start_request, create_start_request_with_type,
                MockTokenManagerWrapper,
            },
            signaling_app,
        },
    };

    #[tokio::test]
    async fn start() {
        let mut tokens = MockTokenManager::new();
        tokens
            .expect_issue::<EdrClaims>()
            .returning(|_: &EdrClaims| Ok("token".to_string()));
        let mut store = MockTransferStore::new();

        store
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let ctx = create_context(tokens, store);
        let app: Router = signaling_app::<MockTokenManagerWrapper>().with_state(ctx);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/dataflows")
                    .method("POST")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&create_start_request()).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK, "{}", body_str);

        let flow_response: DataFlowResponseMessage = serde_json::from_str(&body_str).unwrap();
        let token = flow_response
            .data_address
            .as_ref()
            .and_then(|dad| dad.get_property(&EDC_NAMESPACE.to_iri("access_token")));

        assert_eq!(token, Some("token"));
    }

    #[tokio::test]
    async fn start_fails_with_invalid_datasource() {
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
                    .body(Body::from(
                        serde_json::to_vec(&create_start_request_with_type("FakeType")).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", body_str);

        assert_eq!(
            serde_json::from_str::<Value>(&body_str).unwrap(),
            json!({
            "error": "Invalid Source Data Address"})
        );
    }
}
