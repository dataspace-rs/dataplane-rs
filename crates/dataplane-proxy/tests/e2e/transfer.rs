use std::{collections::HashMap, time::Duration};

use edc_connector_client::types::{
    data_address::DataAddress, transfer_process::TransferProcessState,
};
use edc_dataplane_proxy::model::token::TokenResponse;
use jsonwebtoken::{jwk::JwkSet, Algorithm, DecodingKey, Validation};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use tokio::{sync::OnceCell, time::sleep};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::e2e::{
    launch_data_plane, launch_data_plane_with_token_duration, seed_transfer_process,
    setup_consumer_client, setup_provider_client, wait_for_dataplane, wait_for_transfer_state,
};

fn create_data_address(base_url: String) -> DataAddress {
    DataAddress::builder()
        .kind("HttpData")
        .property("baseUrl", base_url)
        .build()
        .unwrap()
}

static HTTP_CLIENT: OnceCell<Client> = OnceCell::const_new();

async fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| async { Client::new() }).await
}

#[tokio::test]
async fn transfer_pull_test_single() {
    let handle = launch_data_plane().await;
    let consumer = setup_consumer_client();
    let provider = setup_provider_client();
    let mock_server = MockServer::start().await;

    wait_for_dataplane(&provider, handle.component_id()).await;

    let body = json!({
        "name": "Mark",
        "age": 30,
        "city": "London"
    });

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let data_address = create_data_address(mock_server.uri());

    let (transfer_id, ..) = seed_transfer_process(&consumer, &provider, data_address).await;

    let edr = consumer
        .edrs()
        .get_data_address(&transfer_id)
        .await
        .unwrap();

    verify_token(&edr).await;

    let response = fetch_data(&edr).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), body);
}

async fn verify_token(edr: &DataAddress) {
    let jwk_url = edr.property::<String>("jwks_url").unwrap().unwrap();
    let token = edr.property::<String>("access_token").unwrap().unwrap();

    let keys = http_client()
        .await
        .get(jwk_url)
        .send()
        .await
        .unwrap()
        .json::<JwkSet>()
        .await
        .unwrap();

    let header = jsonwebtoken::decode_header(&token).unwrap();

    let key = keys.find(&header.kid.unwrap()).unwrap();

    let decoding_key = DecodingKey::from_jwk(key).unwrap();

    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.validate_aud = false;

    jsonwebtoken::decode::<Value>(&token, &decoding_key, &validation).unwrap();
}

async fn fetch_data(edr: &DataAddress) -> Response {
    let endpoint = edr.property::<String>("endpoint").unwrap().unwrap();
    let access_token = edr.property::<String>("access_token").unwrap().unwrap();

    fetch_data_with_token(&endpoint, &access_token).await
}

async fn fetch_data_with_token(endpoint: &str, access_token: &str) -> Response {
    http_client()
        .await
        .get(endpoint)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap()
}

async fn renew_token(edr: &DataAddress, client_id: &str) -> Response {
    let endpoint = edr.property::<String>("refresh_endpoint").unwrap().unwrap();
    let refresh_token = edr.property::<String>("refresh_token").unwrap().unwrap();

    let mut params = HashMap::new();
    params.insert("refresh_token", refresh_token.as_str());
    params.insert("grant_type", "refresh_token");
    params.insert("client_id", client_id);

    http_client()
        .await
        .post(endpoint)
        .form(&params)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn transfer_pull_test_with_terminate() {
    let handle = launch_data_plane().await;
    let consumer = setup_consumer_client();
    let provider = setup_provider_client();

    wait_for_dataplane(&provider, handle.component_id()).await;

    let data_address = create_data_address("http://localhost:8080".to_string());

    let (transfer_id, ..) = seed_transfer_process(&consumer, &provider, data_address).await;

    let edr = consumer
        .edrs()
        .get_data_address(&transfer_id)
        .await
        .unwrap();

    let tp = consumer
        .transfer_processes()
        .get(&transfer_id)
        .await
        .unwrap();

    provider
        .transfer_processes()
        .terminate(&tp.correlation_id().unwrap(), "termination")
        .await
        .unwrap();

    wait_for_transfer_state(&consumer, &transfer_id, TransferProcessState::Terminated).await;

    let response = fetch_data(&edr).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn transfer_pull_test_with_suspend_and_resume() {
    let handle = launch_data_plane().await;
    let consumer = setup_consumer_client();
    let provider = setup_provider_client();
    let mock_server = MockServer::start().await;

    let body = json!({
        "name": "Mark",
        "age": 30,
        "city": "London"
    });

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    wait_for_dataplane(&provider, handle.component_id()).await;

    let data_address = create_data_address(mock_server.uri());
    let (transfer_id, ..) = seed_transfer_process(&consumer, &provider, data_address).await;

    let edr = consumer
        .edrs()
        .get_data_address(&transfer_id)
        .await
        .unwrap();

    consumer
        .transfer_processes()
        .suspend(&transfer_id, "suspend")
        .await
        .unwrap();

    wait_for_transfer_state(&consumer, &transfer_id, TransferProcessState::Suspended).await;

    let response = fetch_data(&edr).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    consumer
        .transfer_processes()
        .resume(&transfer_id)
        .await
        .unwrap();

    wait_for_transfer_state(&consumer, &transfer_id, TransferProcessState::Started).await;

    let response = fetch_data(&edr).await;

    // Old one not working
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let edr = consumer
        .edrs()
        .get_data_address(&transfer_id)
        .await
        .unwrap();

    let response = fetch_data(&edr).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), body);
}

#[tokio::test]
async fn transfer_pull_test_with_token_expiration() {
    let handle =
        launch_data_plane_with_token_duration(Duration::from_secs(2), Duration::from_secs(40))
            .await;
    let consumer = setup_consumer_client();
    let provider = setup_provider_client();
    let mock_server = MockServer::start().await;

    let body = json!({
        "name": "Mark",
        "age": 30,
        "city": "London"
    });

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    wait_for_dataplane(&provider, handle.component_id()).await;

    let data_address = create_data_address(mock_server.uri());
    let (transfer_id, ..) = seed_transfer_process(&consumer, &provider, data_address).await;

    let edr = consumer
        .edrs()
        .get_data_address(&transfer_id)
        .await
        .unwrap();

    sleep(Duration::from_secs(3)).await;

    let response = fetch_data(&edr).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = renew_token(&edr, "consumer").await;

    assert_eq!(response.status(), StatusCode::OK);

    let new_token = response.json::<TokenResponse>().await.unwrap();
    let endpoint = edr.property::<String>("endpoint").unwrap().unwrap();

    let response = fetch_data_with_token(&endpoint, &new_token.access_token).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), body);
}
