use std::{future::Future, time::Duration};

use dataplane_core::{
    core::model::namespace::EDC_NAMESPACE, default_bind, default_db, default_refresh_token_duration, default_renewal_port, default_signaling_port, default_token_duration, DataPlane, DataPlaneCfg, DataPlaneHandle, KeyFormat, Proxy, ProxyKeys, TokenRenewal, Signaling
};
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

pub async fn launch_data_plane() -> DataPlaneHandle {
    init_dataplane(default_token_duration(), default_refresh_token_duration()).await
}

pub async fn launch_data_plane_with_token_duration(
    duration: Duration,
    refresh_token_expiration: Duration,
) -> DataPlaneHandle {
    init_dataplane(duration.as_secs(), refresh_token_expiration.as_secs()).await
}

async fn init_dataplane(token_expiration: u64, refresh_token_expiration: u64) -> DataPlaneHandle {
    let component_id = Uuid::new_v4();
    let (private_key, public_key) = generate_key_pair();

    let cfg = DataPlaneCfg::builder()
        .db(default_db())
        .proxy(
            Proxy::builder()
                .issuer("issuer")
                .proxy_url("http://localhost:8787/api/v1/public")
                .token_duration(token_expiration)
                .keys(
                    ProxyKeys::builder()
                        .private_key(private_key.into())
                        .public_key(public_key)
                        .format(KeyFormat::Pem)
                        .kid("kid")
                        .algorithm("EdDSA")
                        .build(),
                )
                .refresh_token_duration(refresh_token_expiration)
                .token_leeway(0)
                .renewal(
                    TokenRenewal::builder()
                        .bind(default_bind())
                        .port(default_renewal_port())
                        .build(),
                )
                .build(),
        )
        .signaling(
            Signaling::builder()
                .signaling_url("http://host.docker.internal:8787/api/v1/dataflows")
                .control_plane_url("http://localhost:29192/control")
                .bind(default_bind())
                .port(default_signaling_port())
                .build(),
        )
        .component_id(format!("provider-{}", component_id))
        .build();

    DataPlane::builder()
        .with_config(cfg)
        .prepare()
        .unwrap()
        .start()
        .await
        .unwrap()
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
        .build()
        .unwrap();

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
        .build()
        .unwrap();

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
        .build()
        .unwrap();

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
                .id(&offer_id)
                .kind(PolicyKind::Offer)
                .assigner(PROVIDER_ID)
                .target(Target::id(&asset_id))
                .build(),
        )
        .build()
        .unwrap();

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
        .build()
        .unwrap();

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
