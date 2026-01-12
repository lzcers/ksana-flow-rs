use axum::{extract::State, response::IntoResponse, Json};
use crate::state::AppState;

pub async fn get_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let registry = state.registry.read().expect("registry lock poisoned");
    let nodes = registry.get_metadata();
    Json(nodes)
}
