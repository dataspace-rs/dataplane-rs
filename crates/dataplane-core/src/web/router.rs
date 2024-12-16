use axum::{
    routing::{get, post},
    Router,
};

use crate::core::service::token::TokenManager;

use super::{
    api::{
        dataflows::{health_check, init_flow, suspend_flow, terminate_flow},
        jwks::jwks,
        public::public_handler,
        token::refresh_token,
    },
    state::Context,
};

pub fn signaling_app<T: TokenManager + Send + Sync + Clone + 'static>() -> Router<Context<T>> {
    Router::new()
        .route("/api/v1/dataflows/check", get(health_check))
        .route("/api/v1/dataflows", post(init_flow))
        .route("/api/v1/dataflows/:id/terminate", post(terminate_flow))
        .route("/api/v1/dataflows/:id/suspend", post(suspend_flow))
        .route(
            "/api/v1/public",
            post(public_handler)
                .get(public_handler)
                .delete(public_handler)
                .head(public_handler)
                .patch(public_handler)
                .put(public_handler)
                .options(public_handler),
        )
}

pub fn token_app<T: TokenManager + Send + Sync + Clone + 'static>() -> Router<Context<T>> {
    Router::new()
        .route("/.well-known/jwks.json", get(jwks))
        .route("/api/v1/token", post(refresh_token))
}
