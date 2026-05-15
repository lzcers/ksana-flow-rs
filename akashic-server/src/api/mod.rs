pub mod dto;
pub mod handlers;

use std::time::Instant;

use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use tracing::info;

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/game-sessions", post(handlers::create_game_session))
        .route(
            "/api/game-sessions/{session_id}/control",
            post(handlers::control_game_session),
        )
        .route(
            "/api/game-sessions/{session_id}",
            get(handlers::get_game_session_world),
        )
        .route(
            "/api/game-sessions/{session_id}/stream",
            get(handlers::stream_game_session),
        )
        .route("/healthz", get(handlers::healthz))
        .route_layer(middleware::from_fn(log_api_request))
        .with_state(state)
}

async fn log_api_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let started_at = Instant::now();

    info!("api request started: {} {}", method, uri);
    let response = next.run(request).await;
    let status = response.status();
    let elapsed = started_at.elapsed().as_millis();
    info!(
        "api request finished: {} {} -> {} ({} ms)",
        method, uri, status, elapsed
    );

    response
}
