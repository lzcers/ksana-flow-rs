pub mod dto;
pub mod handlers;

use axum::{
    Router,
    routing::{get, post},
};

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
        .with_state(state)
}
