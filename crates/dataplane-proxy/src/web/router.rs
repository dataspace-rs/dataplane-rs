use axum::{
    routing::{get, post},
    Router,
};

use crate::service::token::TokenManager;

use super::{
    api::{jwks::jwks, token::refresh_token},
    state::Context,
};

pub fn token_app<T: TokenManager + Send + Sync + Clone + 'static>() -> Router<Context<T>> {
    Router::new()
        .route("/.well-known/jwks.json", get(jwks))
        .route("/api/v1/token", post(refresh_token))
}
