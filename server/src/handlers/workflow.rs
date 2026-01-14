use crate::state::{AppState, GraphBlueprint};
use axum::{
    Json,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use flow::{FlowEvent, Runner, SendableAny};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    name: String,
    blueprint: GraphBlueprint,
}

#[derive(Deserialize)]
pub struct UpdateWorkflowRequest {
    name: Option<String>,
    blueprint: GraphBlueprint,
}

#[derive(Deserialize)]
pub struct RunNodeRequest {
    blueprint: GraphBlueprint,
    node_id: String,
}

pub async fn list_workflows(State(state): State<AppState>) -> impl IntoResponse {
    let workflows = {
        let db = state.db.lock().expect("db lock poisoned");
        db.list_workflows().unwrap_or_default()
    };
    let response: Vec<_> = workflows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Json(response)
}

pub async fn get_workflow(Path(id): Path<i64>, State(state): State<AppState>) -> impl IntoResponse {
    let workflow = {
        let db = state.db.lock().expect("db lock poisoned");
        db.get_workflow(id).unwrap_or(None)
    };

    if let Some((name, blueprint)) = workflow {
        Json(json!({ "id": id, "name": name, "blueprint": blueprint }))
    } else {
        Json(json!({ "error": "Workflow not found" }))
    }
}

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> impl IntoResponse {
    let result = {
        let db = state.db.lock().expect("db lock poisoned");
        let blueprint_value = serde_json::to_value(&payload.blueprint).unwrap();
        db.create_workflow(&payload.name, &blueprint_value)
    };

    match result {
        Ok(id) => Json(json!({ "id": id, "status": "created" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn update_workflow(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> impl IntoResponse {
    let result = {
        let db = state.db.lock().expect("db lock poisoned");
        let blueprint_value = serde_json::to_value(&payload.blueprint).unwrap();
        let name = payload.name.unwrap_or_else(|| "Untitled".to_string());
        db.update_workflow(id, &name, &blueprint_value)
    };

    match result {
        Ok(true) => Json(json!({ "status": "updated" })),
        Ok(false) => Json(json!({ "error": "Workflow not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn delete_workflow(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = {
        let db = state.db.lock().expect("db lock poisoned");
        db.delete_workflow(id)
    };

    match result {
        Ok(true) => Json(json!({ "status": "deleted" })),
        Ok(false) => Json(json!({ "error": "Workflow not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn run_workflow(
    State(state): State<AppState>,
    Json(blueprint): Json<GraphBlueprint>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Workflow is already running"}));
    }

    let (graph, start_inputs) = {
        let registry = state
            .registry
            .read()
            .expect("Failed to read registry: lock poisoned");

        let (graph, start_nodes) = match blueprint.instantiate(&registry) {
            Ok(v) => v,
            Err(e) => return Json(json!({"error": e})),
        };

        let mut inputs = Vec::new();
        for node_id in &start_nodes {
            if let Some(node) = blueprint.nodes.iter().find(|n| n.id == *node_id) {
                if let Some(meta) = registry.get_node_metadata(&node.type_name) {
                    inputs.push((
                        node_id.clone(),
                        crate::registry::create_default_value(&meta.inputs),
                    ));
                } else {
                    inputs.push((node_id.clone(), Box::new(()) as Box<dyn SendableAny>));
                }
            }
        }
        (graph, inputs)
    };

    state.set_running(true);
    let tx = state.tx.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);
        let bridge_tx = tx.clone();
        let bridge_handle = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let _ = bridge_tx.send(event);
            }
        });

        {
            let mut runner = Runner::new(graph).set_event_sender(event_tx);

            for (node_id, input) in start_inputs {
                runner = runner.set_start_node(&node_id, input.as_ref());
            }

            if let Err(e) = runner.run().await {
                tracing::error!("Flow execution error: {}", e);
                let _ = tx.send(FlowEvent::NodeError("runner".to_string(), e));
            }
        }

        // Wait for all events to be sent before finishing
        let _ = bridge_handle.await;
        state_clone.set_running(false);
        let _ = tx.send(FlowEvent::Finished);
    });

    Json(json!({"status": "started"}))
}

pub async fn run_node(
    State(state): State<AppState>,
    Json(payload): Json<RunNodeRequest>,
) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Workflow is already running"}));
    }

    let (graph, start_input) = {
        let registry = state
            .registry
            .read()
            .expect("Failed to read registry: lock poisoned");

        let (graph, _) = match payload.blueprint.instantiate(&registry) {
            Ok(v) => v,
            Err(e) => return Json(json!({"error": e})),
        };

        let input = if let Some(node) = payload
            .blueprint
            .nodes
            .iter()
            .find(|n| n.id == payload.node_id)
        {
            if let Some(meta) = registry.get_node_metadata(&node.type_name) {
                crate::registry::create_default_value(&meta.inputs)
            } else {
                Box::new(()) as Box<dyn SendableAny>
            }
        } else {
            Box::new(()) as Box<dyn SendableAny>
        };

        (graph, input)
    };

    if !graph.nodes.contains_key(&payload.node_id) {
        return Json(json!({
            "error": format!("Node '{}' not found in blueprint", payload.node_id)
        }));
    }

    state.set_running(true);
    let tx = state.tx.clone();
    let state_clone = state.clone();
    let start_node_id = payload.node_id.clone();

    tokio::spawn(async move {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);
        let bridge_tx = tx.clone();
        let bridge_handle = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let _ = bridge_tx.send(event);
            }
        });

        {
            let mut runner = Runner::new(graph).set_event_sender(event_tx);
            runner = runner.set_start_node(&start_node_id, start_input.as_ref());

            if let Err(e) = runner.run().await {
                tracing::error!("Flow execution error: {}", e);
                let _ = tx.send(FlowEvent::NodeError("runner".to_string(), e));
            }
        }

        // Wait for all events to be sent before finishing
        let _ = bridge_handle.await;
        state_clone.set_running(false);
        let _ = tx.send(FlowEvent::Finished);
    });

    Json(json!({"status": "started", "start_node": payload.node_id}))
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}
