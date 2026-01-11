use crate::state::{AppState, NodeConfig, Position};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde_json::json;

pub async fn get_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let registry = state
        .registry
        .read()
        .expect("Failed to read registry: lock poisoned");
    Json(registry.get_metadata())
}

pub async fn add_node(
    State(state): State<AppState>,
    Json(config): Json<NodeConfig>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Cannot edit nodes while workflow is running"}));
    }

    let registry = state
        .registry
        .read()
        .expect("Failed to read registry: lock poisoned");
    match registry.create_node(&config.type_name, config.config.clone()) {
        Ok(_) => {
            // Update Blueprint
            {
                let mut blueprint = state
                    .blueprint
                    .write()
                    .expect("Failed to write blueprint: lock poisoned");
                // Check if exists
                if let Some(idx) = blueprint.nodes.iter().position(|n| n.id == config.id) {
                    blueprint.nodes[idx] = config;
                } else {
                    blueprint.nodes.push(config);
                }
            }
            Json(json!({"status": "ok"}))
        }
        Err(e) => Json(json!({"error": e})),
    }
}

pub async fn update_node_position(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(position): Json<Position>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Cannot edit nodes while workflow is running"}));
    }

    let mut blueprint = state
        .blueprint
        .write()
        .expect("Failed to write blueprint: lock poisoned");

    if let Some(node) = blueprint.nodes.iter_mut().find(|n| n.id == id) {
        node.position = Some(position);
        Json(json!({"status": "ok"}))
    } else {
        Json(json!({"error": "Node not found"}))
    }
}

pub async fn remove_node(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Cannot remove nodes while workflow is running"}));
    }

    // Update Blueprint
    {
        let mut blueprint = state
            .blueprint
            .write()
            .expect("Failed to write blueprint: lock poisoned");
        blueprint.nodes.retain(|n| n.id != id);
        blueprint.edges.retain(|e| e.source != id && e.target != id);
    }
    Json(json!({"status": "ok"}))
}
