use crate::state::{AppState, EdgeConfig};
use axum::{Json, extract::State, response::IntoResponse};
use flow::{AnyEdge, Edge};
use nodes::trade::K;
use nodes::trade::backtester::BacktesterInput;
use serde_json::json;

pub async fn get_graph(State(state): State<AppState>) -> impl IntoResponse {
    let blueprint = state
        .blueprint
        .read()
        .expect("Failed to read blueprint: lock poisoned");
    Json(blueprint.clone())
}

pub async fn add_edge(
    State(state): State<AppState>,
    Json(config): Json<EdgeConfig>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Cannot edit graph while workflow is running"}));
    }

    let blueprint = state
        .blueprint
        .read()
        .expect("Failed to read blueprint: lock poisoned");
    // Find source node type
    let source_node = blueprint.nodes.iter().find(|n| n.id == config.source);

    if let Some(source_node) = source_node {
        let type_name = &source_node.type_name;
        // Determine edge type based on node type
        let edge_valid = match type_name.as_str() {
            "ReactiveSourceNode" | "VOLMFINode" | "Backtester" => true,
            _ => false,
        };

        if edge_valid {
            drop(blueprint);
            let mut blueprint = state
                .blueprint
                .write()
                .expect("Failed to write blueprint: lock poisoned");
            blueprint.edges.push(config);

            Json(json!({"status": "ok"}))
        } else {
            Json(json!({"error": "Unknown source node type or unsupported edge"}))
        }
    } else {
        Json(json!({"error": "Source node not found"}))
    }
}

pub async fn remove_edge(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Cannot remove edges while workflow is running"}));
    }

    let mut blueprint = state
        .blueprint
        .write()
        .expect("Failed to write blueprint: lock poisoned");
    if let Some(idx) = blueprint.edges.iter().position(|e| e.id == id) {
        blueprint.edges.remove(idx);
        Json(json!({"status": "ok"}))
    } else {
        Json(json!({"error": "Edge not found"}))
    }
}
