use edc_dataplane_core::core::model::namespace::EDC_NAMESPACE;
use miwa::{
    core::{Extension, ExtensionConfig, MiwaContext, MiwaResult},
    derive::{extension, ExtensionConfig},
};
use reqwest::Response;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, error, info};

pub struct RegistrationExtension {
    component_id: String,
    cfg: SignalingConfig,
}

impl RegistrationExtension {
    pub fn new(component_id: String, cfg: SignalingConfig) -> Self {
        Self { component_id, cfg }
    }
}

#[async_trait::async_trait]
impl Extension for RegistrationExtension {
    async fn start(&self) -> MiwaResult<()> {
        let component_id = self.component_id.clone();
        let cfg = self.cfg.clone();
        tokio::task::spawn(async {
            register_dataplane(component_id, cfg).await;
        });
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[derive(Deserialize, ExtensionConfig, Clone)]
#[config(prefix = "signaling")]
pub struct SignalingConfig {
    control_plane_url: String,
    signaling_url: String,
    transfer_types: Vec<String>,
    source_types: Vec<String>,
}

#[extension(name = "Data plane registration extension")]
pub async fn registration_extension(
    ctx: &MiwaContext,
    ExtensionConfig(cfg): ExtensionConfig<SignalingConfig>,
) -> MiwaResult<RegistrationExtension> {
    Ok(RegistrationExtension::new(
        ctx.component_id().to_string(),
        cfg,
    ))
}

pub async fn register_dataplane(component_id: String, cfg: SignalingConfig) {
    loop {
        debug!(
            "Registering dataplane with control plane: {}",
            cfg.control_plane_url
        );

        let response = send_registration(&component_id, &cfg).await;

        let error = match response {
            Ok(response) if response.status().is_success() => {
                info!(
                    "Registered dataplane: {:?} at {:?}",
                    component_id, cfg.control_plane_url
                );
                break;
            }
            Ok(response) => response.text().await.ok().unwrap_or_default(),
            Err(e) => e.to_string(),
        };

        error!("Failed to register dataplane: {}", error);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

async fn send_registration(component_id: &str, cfg: &SignalingConfig) -> anyhow::Result<Response> {
    const PATH: &str = "/v1/dataplanes";
    reqwest::Client::new()
        .post(format!("{}{}", cfg.control_plane_url, PATH))
        .json(&json!({
            "@context" : {
                "@vocab": EDC_NAMESPACE.ns()
            },
            "@id": component_id,
            "url": cfg.signaling_url,
            "allowedTransferTypes": cfg.transfer_types,
            "allowedSourceTypes": cfg.source_types,
        }))
        .send()
        .await
        .map(Ok)?
}
