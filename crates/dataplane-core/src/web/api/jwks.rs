use axum::{extract::State, response::IntoResponse, Json};
use jsonwebtoken::jwk::JwkSet;
use reqwest::StatusCode;
use serde_json::json;

use crate::{
    core::service::token::{TokenError, TokenManager},
    web::state::Context,
};

pub async fn jwks<T: TokenManager + Clone>(
    State(ctx): State<Context<T>>,
) -> Result<Json<JwkSet>, JwkError> {
    ctx.tokens().keys().map_err(JwkError::Token).map(Json)
}

pub enum JwkError {
    Token(TokenError),
}

impl IntoResponse for JwkError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            JwkError::Token(_e) => (StatusCode::BAD_GATEWAY, "Bad gateway"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
