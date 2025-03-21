use axum::{
    extract::{Path, State},
    Json,
};
use edc_dataplane_core::{
    core::service::transfer::TransferService,
    signaling::{
        DataFlowResponseMessage, DataFlowStartMessage, DataFlowSuspendMessage,
        DataFlowTerminateMessage,
    },
};
use serde_json::{json, Value};

use crate::web::{context::WithContext, error::SignalingResult};

pub async fn health_check() -> SignalingResult<Json<Value>> {
    Ok(Json(json!({"status": "ok"})))
}

pub async fn init_flow(
    State(manager): State<TransferService>,
    Json(flow): Json<DataFlowStartMessage>,
) -> SignalingResult<Json<WithContext<DataFlowResponseMessage>>> {
    let response = manager.start(flow).await?;

    Ok(Json(WithContext::builder(response).build()?))
}

pub async fn terminate_flow(
    State(manager): State<TransferService>,
    Path(id): Path<String>,
    Json(msg): Json<DataFlowTerminateMessage>,
) -> SignalingResult<()> {
    manager.terminate(id, msg.reason).await?;

    Ok(())
}

pub async fn suspend_flow(
    State(manager): State<TransferService>,
    Path(id): Path<String>,
    Json(_msg): Json<DataFlowSuspendMessage>,
) -> SignalingResult<()> {
    manager.suspend(id).await?;

    Ok(())
}
