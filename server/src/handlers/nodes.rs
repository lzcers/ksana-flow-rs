use crate::state::AppState;
use axum::{Json, extract::State, response::IntoResponse};

pub async fn get_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let nodes = state.registry.get_metadata();
    Json(nodes)
}
