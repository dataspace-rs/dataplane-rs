use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form, Json,
};
use reqwest::StatusCode;
use serde_json::json;
use tracing::error;

use crate::{
    core::{
        model::token::{TokenRequest, TokenResponse},
        service::token::TokenManager,
    },
    web::state::Context,
};

pub async fn refresh_token<T: TokenManager + Clone>(
    State(ctx): State<Context<T>>,
    Form(request): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, TokenError> {
    ctx.refresh_manager()
        .refresh_token(request)
        .await
        .map(Json)
        .map_err(|err| {
            error!("Failed to refresh token error: {}", err);
            TokenError::InvalidRefreshToken
        })
}

pub enum TokenError {
    InvalidRefreshToken,
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            TokenError::InvalidRefreshToken => (StatusCode::BAD_REQUEST, "Wrong credentials"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
