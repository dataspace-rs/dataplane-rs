use axum::{
    response::{IntoResponse, Response},
    Json,
};
use reqwest::StatusCode;
use serde_json::json;
use tracing::error;

pub type SignalingResult<T> = Result<T, SignalingError>;

pub enum SignalingError {
    Generic(anyhow::Error),
}

impl IntoResponse for SignalingError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SignalingError::Generic(e) => {
                error!("Internal server error: {:#}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for SignalingError {
    fn from(value: anyhow::Error) -> Self {
        SignalingError::Generic(value)
    }
}
