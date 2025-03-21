use axum::{
    routing::{get, post},
    Router,
};

use super::{
    api::dataflows::{health_check, init_flow, suspend_flow, terminate_flow},
    state::Context,
};

pub fn signaling_app() -> Router<Context> {
    Router::new()
        .route("/api/v1/dataflows/check", get(health_check))
        .route("/api/v1/dataflows", post(init_flow))
        .route("/api/v1/dataflows/:id/terminate", post(terminate_flow))
        .route("/api/v1/dataflows/:id/suspend", post(suspend_flow))
}
