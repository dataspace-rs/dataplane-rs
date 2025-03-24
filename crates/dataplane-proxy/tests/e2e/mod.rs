use std::{future::Future, time::Duration};

use ed25519_compact::{KeyPair, Seed};
use edc_connector_client::{
    types::{
        asset::NewAsset,
        catalog::DatasetRequest,
        contract_definition::NewContractDefinition,
        contract_negotiation::{ContractNegotiationState, ContractRequest},
        data_address::DataAddress,
        dataplane::DataPlaneInstanceState,
        policy::{NewPolicyDefinition, Policy, PolicyKind, Target},
        query::Criterion,
        transfer_process::{TransferProcessState, TransferRequest},
    },
    Auth, EdcConnectorClient,
};
use edc_dataplane_core::core::model::namespace::EDC_NAMESPACE;
use edc_dataplane_proxy::extensions::{proxy_api_extension, proxy_sql_repo_extension};

use edc_dataplane_core::extensions::{sql_repo_extension, transfer_service_extension};
use edc_dataplane_proxy::extensions::transfer_proxy_extension;
use edc_dataplane_signaling::extensions::{registration_extension, signaling_api_extension};
use miwa::core::{Miwa, MiwaHandle};
use serde_json::{json, Value};
use tokio::time::sleep;
use uuid::Uuid;

mod transfer;

pub const PROVIDER_PROTOCOL: &str = "http://provider-connector:9194/protocol";
pub const PROVIDER_ID: &str = "provider";

pub fn setup_provider_client() -> EdcConnectorClient {
    EdcConnectorClient::builder()
        .management_url("http://localhost:29193/management")
        .with_auth(Auth::api_token("123456"))
        .build()
        .unwrap()
}

fn generate_key_pair() -> (String, String) {
    let key_pair = KeyPair::from_seed(Seed::default());
    (key_pair.sk.to_pem(), key_pair.pk.to_pem())
}

pub async fn launch_data_plane() -> MiwaHandle {
    init_dataplane(60 * 10, 60 * 60 * 24 * 30).await
}

pub async fn launch_data_plane_with_token_duration(
    duration: Duration,
    refresh_token_expiration: Duration,
) -> MiwaHandle {
    init_dataplane(duration.as_secs(), refresh_token_expiration.as_secs()).await
}

async fn init_dataplane(token_expiration: u64, refresh_token_expiration: u64) -> MiwaHandle {
    Miwa::prepare()
        .with_json(runtime_config(token_expiration, refresh_token_expiration))
        .build()
        .unwrap()
        .add_extension(sql_repo_extension)
        .add_extension(proxy_sql_repo_extension)
        .add_extension(transfer_service_extension)
        .add_extension(transfer_proxy_extension)
        .add_extension(registration_extension)
        .add_extension(signaling_api_extension)
        .add_extension(proxy_api_extension)
        .start()
        .await
        .unwrap()
}

fn runtime_config(token_expiration: u64, refresh_token_expiration: u64) -> Value {
    let component_id = Uuid::new_v4();
    let (private_key, public_key) = generate_key_pair();
    json!({
        "component_id": component_id,
        "signaling": {
            "control_plane_url": "http://localhost:29192/control",
            "signaling_url": "http://host.docker.internal:8787/api/v1/dataflows",
            "transfer_types":["HttpData-PULL"],
            "source_types":["HttpData"]
        },
        "proxy": {
            "issuer": "issuer",
            "token_duration": token_expiration,
            "refresh_token_duration": refresh_token_expiration,
            "token_leeway": 0,
            "keys": {
                "kid": "kid",
                "algorithm": "EdDSA",
                "format": "Pem",
                "private_key": private_key,
                "public_key": public_key,
            }
        },
        "db": {
            "transfers":{
                "sqlite": {
                    "path": ":memory:"
                }
            },
            "tokens":{
                "sqlite": {
                    "path": ":memory:"
                }
            }
        }
    })
}

pub fn setup_consumer_client() -> EdcConnectorClient {
    EdcConnectorClient::builder()
        .management_url("http://localhost:19193/management")
        .with_auth(Auth::api_token("123456"))
        .build()
        .unwrap()
}

pub async fn seed(
    client: &EdcConnectorClient,
    data_address: DataAddress,
) -> (String, String, String) {
    let asset = NewAsset::builder()
        .id(Uuid::new_v4().to_string().as_str())
        .data_address(data_address)
        .build();

    let asset_response = client.assets().create(&asset).await.unwrap();

    let policy_definition = NewPolicyDefinition::builder()
        .id(Uuid::new_v4().to_string().as_str())
        .policy(Policy::builder().build())
        .build();

    let policy_response = client.policies().create(&policy_definition).await.unwrap();

    let contract_definition = NewContractDefinition::builder()
        .id(Uuid::new_v4().to_string().as_str())
        .asset_selector(Criterion::new(
            &EDC_NAMESPACE.to_iri("id"),
            "=",
            asset_response.id(),
        ))
        .access_policy_id(policy_response.id())
        .contract_policy_id(policy_response.id())
        .build();

    let definition_response = client
        .contract_definitions()
        .create(&contract_definition)
        .await
        .unwrap();

    (
        asset_response.id().to_string(),
        policy_response.id().to_string(),
        definition_response.id().to_string(),
    )
}

pub async fn seed_contract_negotiation(
    consumer: &EdcConnectorClient,
    provider: &EdcConnectorClient,
    data_address: DataAddress,
) -> (String, String) {
    let (asset_id, _, _) = seed(&provider, data_address).await;

    let dataset_request = DatasetRequest::builder()
        .counter_party_address(PROVIDER_PROTOCOL)
        .id(&asset_id)
        .build();

    let dataset = consumer
        .catalogue()
        .dataset(&dataset_request)
        .await
        .unwrap();

    let offer_id = dataset.offers()[0].id().unwrap();

    let request = ContractRequest::builder()
        .counter_party_address(PROVIDER_PROTOCOL)
        .counter_party_id(PROVIDER_ID)
        .policy(
            Policy::builder()
                .id(offer_id)
                .kind(PolicyKind::Offer)
                .assigner(PROVIDER_ID)
                .target(Target::id(&asset_id))
                .build(),
        )
        .build();

    let response = consumer
        .contract_negotiations()
        .initiate(&request)
        .await
        .unwrap();

    (response.id().to_string(), asset_id)
}

async fn wait_for_dataplane(provider: &EdcConnectorClient, id: &str) {
    wait_for(|| async {
        provider
            .data_planes()
            .list()
            .await
            .map_err(|err| err.to_string())
            .and_then(|dataplanes| {
                dataplanes
                    .iter()
                    .find(|dp| dp.state() == &DataPlaneInstanceState::Available && dp.id() == id)
                    .map(|_| ())
                    .ok_or("No dataplanes found".to_string())
            })
    })
    .await
    .unwrap();
}

pub async fn seed_transfer_process(
    consumer: &EdcConnectorClient,
    provider: &EdcConnectorClient,
    data_address: DataAddress,
) -> (String, String, String, String) {
    let (contract_negotiation_id, asset_id) =
        seed_contract_negotiation(consumer, provider, data_address).await;

    wait_for_negotiation_state(
        &consumer,
        &contract_negotiation_id,
        ContractNegotiationState::Finalized,
    )
    .await;

    let agreement_id = consumer
        .contract_negotiations()
        .get(&contract_negotiation_id)
        .await
        .map(|cn| cn.contract_agreement_id().cloned())
        .unwrap()
        .unwrap();

    let contract_agreement = consumer
        .contract_agreements()
        .get(&agreement_id)
        .await
        .unwrap();

    let request = TransferRequest::builder()
        .counter_party_address(PROVIDER_PROTOCOL)
        .contract_id(&agreement_id)
        .transfer_type("HttpData-PULL")
        .destination(DataAddress::builder().kind("HttpProxy").build().unwrap())
        .build();

    let response = consumer
        .transfer_processes()
        .initiate(&request)
        .await
        .unwrap();

    wait_for_transfer_state(&consumer, response.id(), TransferProcessState::Started).await;

    (
        response.id().to_string(),
        contract_agreement.id().to_string(),
        contract_negotiation_id,
        asset_id,
    )
}

pub async fn wait_for_transfer_state(
    client: &EdcConnectorClient,
    id: &str,
    state: TransferProcessState,
) {
    wait_for(|| {
        let i_state = state.clone();
        async {
            client
                .transfer_processes()
                .get_state(id)
                .await
                .map_err(|err| err.to_string())
                .and_then(|s| {
                    if s == state {
                        Ok(i_state)
                    } else {
                        Err("State mismatch".to_string())
                    }
                })
        }
    })
    .await
    .unwrap();
}

pub async fn wait_for_negotiation_state(
    client: &EdcConnectorClient,
    id: &str,
    state: ContractNegotiationState,
) {
    wait_for(|| {
        let i_state = state.clone();
        async {
            client
                .contract_negotiations()
                .get_state(id)
                .await
                .map_err(|err| err.to_string())
                .and_then(|s| {
                    if s == state {
                        Ok(i_state)
                    } else {
                        Err("State mismatch".to_string())
                    }
                })
        }
    })
    .await
    .unwrap();
}

pub async fn wait_for<F, Fut, R, E>(f: F) -> Result<R, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<R, E>>,
{
    let timeout = tokio::time::timeout(Duration::from_secs(30), async move {
        loop {
            match f().await {
                Ok(r) => break Ok(r),
                Err(_) => {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        }
    });

    timeout.await.unwrap()
}
