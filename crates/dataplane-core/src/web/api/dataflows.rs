use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::{
    core::service::{
        token::TokenManager,
        transfer::{SignalingError, SignalingResult, TransferManager},
    },
    signaling::{
        DataFlowResponseMessage, DataFlowStartMessage, DataFlowSuspendMessage,
        DataFlowTerminateMessage,
    },
    web::context::WithContext,
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

impl IntoResponse for SignalingError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SignalingError::InvalidSourceDataAddress(_) => {
                (StatusCode::BAD_REQUEST, "Invalid Source Data Address")
            }
            SignalingError::GenericError(_error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            SignalingError::EdrError(_edr_error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error generating EDR")
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
