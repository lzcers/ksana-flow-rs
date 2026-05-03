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
            "/api/game-sessions/{session_id}",
            get(handlers::get_game_session),
        )
        .route(
            "/api/game-sessions/{session_id}/choices",
            post(handlers::submit_choice),
        )
        .route(
            "/api/game-sessions/{session_id}/ending",
            get(handlers::get_game_session_ending),
        )
        .route(
            "/api/game-sessions/{session_id}/intuition-preview",
            post(handlers::create_intuition_preview),
        )
        .route(
            "/api/game-sessions/{session_id}/history",
            get(handlers::get_game_session_history),
        )
        .route(
            "/api/game-sessions/{session_id}/stream",
            get(handlers::stream_game_session),
        )
        .route(
            "/api/saves",
            get(handlers::list_saves).post(handlers::create_save),
        )
        .route("/api/saves/{save_id}/load", post(handlers::load_save))
        .route("/api/archives", get(handlers::list_archives))
        .route("/api/archives/{archive_id}", get(handlers::get_archive))
        .route("/api/share/save-card", post(handlers::generate_save_share_card))
        .route(
            "/api/share/ending-card",
            post(handlers::generate_ending_share_card),
        )
        .route("/healthz", get(handlers::healthz))
        .with_state(state)
}
