use reqwest::Response;
use serde_json::json;
use tracing::{debug, error, info};

use crate::{core::model::namespace::EDC_NAMESPACE, DataPlaneCfg};

pub async fn register_dataplane(cfg: DataPlaneCfg) {
    loop {
        debug!(
            "Registering dataplane with control plane: {}",
            cfg.signaling.control_plane_url
        );

        let response = send_registration(&cfg).await;

        let error = match response {
            Ok(response) if response.status().is_success() => {
                info!(
                    "Registered dataplane: {:?} at {:?}",
                    cfg.component_id, cfg.signaling.control_plane_url
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

async fn send_registration(cfg: &DataPlaneCfg) -> anyhow::Result<Response> {
    const PATH: &str = "/v1/dataplanes";
    reqwest::Client::new()
        .post(format!("{}{}", cfg.signaling.control_plane_url, PATH))
        .json(&json!({
            "@context" : {
                "@vocab": EDC_NAMESPACE.ns()
            },
            "@id": cfg.component_id,
            "url": cfg.signaling.signaling_url,
            "allowedTransferTypes": ["HttpData-PULL"],
            "allowedSourceTypes": ["HttpData"],
            "allowedDestTypes": ["HttpData"]
        }))
        .send()
        .await
        .map(Ok)?
}
