use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::{
    core::service::{token::TokenManager, transfer::TransferManager},
    signaling::{
        DataFlowResponseMessage, DataFlowStartMessage, DataFlowSuspendMessage,
        DataFlowTerminateMessage,
    },
    web::{context::WithContext, error::SignalingResult},
};

pub async fn health_check() -> SignalingResult<Json<Value>> {
    Ok(Json(json!({"status": "ok"})))
}

pub async fn init_flow<T: TokenManager>(
    State(manager): State<TransferManager<T>>,
    Json(flow): Json<DataFlowStartMessage>,
) -> SignalingResult<Json<WithContext<DataFlowResponseMessage>>> {
    let response = manager.start(flow).await?;

    Ok(Json(WithContext::builder(response).build()?))
}

pub async fn terminate_flow<T: TokenManager>(
    State(manager): State<TransferManager<T>>,
    Path(id): Path<String>,
    Json(msg): Json<DataFlowTerminateMessage>,
) -> SignalingResult<()> {
    manager.terminate(id, msg.reason).await?;

    Ok(())
}

pub async fn suspend_flow<T: TokenManager>(
    State(manager): State<TransferManager<T>>,
    Path(id): Path<String>,
    Json(_msg): Json<DataFlowSuspendMessage>,
) -> SignalingResult<()> {
    manager.suspend(id).await?;

    Ok(())
}
