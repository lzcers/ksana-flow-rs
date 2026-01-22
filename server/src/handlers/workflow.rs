use crate::{
    registry,
    state::{AppState, ExecutionHandle, GraphBlueprint},
};
use axum::{
    Json,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use flow::{
    FlowEvent, Runner, SendableAny,
    context::{ExecutionContext, NodeState},
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub space_id: String,
    name: String,
    blueprint: GraphBlueprint,
}

#[derive(Deserialize)]
pub struct UpdateWorkflowRequest {
    pub space_id: String,
    name: Option<String>,
    blueprint: GraphBlueprint,
}

#[derive(Deserialize)]
pub struct RunWorkflowRequest {
    pub space_id: String,
    blueprint: GraphBlueprint,
    workflow_id: i64,
}

#[derive(Deserialize)]
pub struct RunNodeRequest {
    pub space_id: String,
    blueprint: GraphBlueprint,
    node_id: String,
}

#[derive(Deserialize)]
pub struct WorkflowQueryParams {
    pub space_id: String,
}

#[derive(Deserialize)]
pub struct WsParams {
    pub workspace_id: String,
}

pub async fn list_workflows(
    Query(params): Query<WorkflowQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let workflows = {
        let db = state.db.lock().expect("db lock poisoned");
        db.list_workflows(&params.space_id).unwrap_or_default()
    };
    let response: Vec<_> = workflows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Json(response)
}

pub async fn get_workflow(
    Path(id): Path<i64>,
    Query(params): Query<WorkflowQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let workflow = {
        let db = state.db.lock().expect("db lock poisoned");
        db.get_workflow(id, &params.space_id).unwrap_or(None)
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
        db.create_workflow(&payload.name, &blueprint_value, &payload.space_id)
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
        db.update_workflow(id, &name, &blueprint_value, &payload.space_id)
    };

    match result {
        Ok(true) => Json(json!({ "status": "updated" })),
        Ok(false) => Json(json!({ "error": "Workflow not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn delete_workflow(
    Path(id): Path<i64>,
    Query(params): Query<WorkflowQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = {
        let db = state.db.lock().expect("db lock poisoned");
        db.delete_workflow(id, &params.space_id)
    };

    match result {
        Ok(true) => Json(json!({ "status": "deleted" })),
        Ok(false) => Json(json!({ "error": "Workflow not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

fn restore_value(v: Value) -> Box<dyn SendableAny> {
    match v {
        Value::Null => Box::new(()),
        Value::Bool(b) => Box::new(b),
        Value::String(s) => Box::new(s),
        // Keep others as Value
        other => Box::new(other),
    }
}

pub async fn run_workflow(
    State(state): State<AppState>,
    Json(request): Json<RunWorkflowRequest>,
) -> impl IntoResponse {
    let blueprint = request.blueprint;
    let workflow_id = request.workflow_id;
    let workspace_id = request.space_id;
    let run_id = Uuid::new_v4().to_string();

    // Create execution in DB
    {
        let db = state.db.lock().expect("db lock poisoned");
        let _ = db.create_execution(&run_id, workflow_id);
    }

    let (graph, start_inputs) = {
        let registry = &state.registry;

        // 解析 JSON 实例化整个蓝图
        let (graph, start_nodes) = match blueprint.instantiate(registry) {
            Ok(v) => v,
            Err(e) => return Json(json!({"error": e})),
        };

        let mut inputs = Vec::new();
        for node_id in &start_nodes {
            if let Some(node) = blueprint.nodes.iter().find(|n| n.id == *node_id) {
                if let Some(meta) = registry.get_node_metadata(&node.type_name) {
                    inputs.push((
                        node_id.clone(),
                        registry::create_default_value(&meta.inputs),
                    ));
                } else {
                    inputs.push((node_id.clone(), Box::new(()) as Box<dyn SendableAny>));
                }
            }
        }
        (graph, inputs)
    };

    // Reconstruct ExecutionContext from blueprint
    let execution_ctx = ExecutionContext::new();
    for node in &blueprint.nodes {
        if let Some(status) = &node.data.status {
            let state = match status.as_str() {
                "completed" => NodeState::Completed,
                "failed" => NodeState::Failed,
                "running" => NodeState::Running,
                "skipped" => NodeState::Skipped,
                "idle" => NodeState::Idle,
                _ => NodeState::Pending,
            };
            execution_ctx.set_state(node.id.clone(), state);

            if state == NodeState::Completed && !node.data.outputs.is_null() {
                let val = node.data.outputs.clone();
                let output = restore_value(val);
                execution_ctx.set_output(node.id.clone(), output);
            }
        }
    }

    // Prepare Runner
    let (mut runner, handle) = Runner::new(graph, Some(execution_ctx));

    // Setup bridge
    let tx = state.tx.clone();
    let run_id_clone = run_id.clone();
    let workspace_id_clone = workspace_id.clone();
    let db_clone = state.db.clone();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    runner.set_event_sender(event_tx);

    let bridge_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            // Save event to DB
            {
                if let Ok(db) = db_clone.lock() {
                    let _ = db.add_execution_event(&run_id_clone, &event);
                }
            }
            // Broadcast
            let _ = tx.send((workspace_id_clone.clone(), run_id_clone.clone(), event));
        }
    });

    // Store execution handle
    {
        let mut executions = state.executions.write().expect("lock poisoned");
        executions.insert(
            run_id.clone(),
            ExecutionHandle {
                runner_handle: handle.clone(),
                workflow_id,
                workspace_id: workspace_id.clone(),
            },
        );
    }

    let state_clone = state.clone();
    let run_id_for_task = run_id.clone();
    let run_id_for_event = run_id.clone();
    let workspace_id_for_event = workspace_id.clone();

    tokio::spawn(async move {
        // Setup inputs
        for (node_id, input) in start_inputs {
            runner.set_start_node(&node_id, input.as_ref());
        }

        if let Err(e) = runner.run().await {
            tracing::error!("Flow execution error: {}", e);
            let _ = state_clone.tx.send((
                workspace_id_for_event.clone(),
                run_id_for_event.clone(),
                FlowEvent::NodeError("runner".to_string(), e),
            ));
        }

        let _ = bridge_handle.await;

        // Remove from executions
        {
            let mut executions = state_clone.executions.write().expect("lock poisoned");
            executions.remove(&run_id_for_task);
        }
    });

    Json(json!({"status": "started", "run_id": run_id}))
}

pub async fn get_workflow_status(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db_status = {
        let db = state.db.lock().expect("db lock poisoned");
        db.get_latest_execution(id).unwrap_or(None)
    };

    if let Some((run_id, status, events)) = db_status {
        // 只返回运行中的
        if status == "running" {
            return Json(json!({
                "status": status,
                "run_id": run_id,
                "events": events
            }));
        }
    }

    Json(json!({
        "status": "idle",
        "run_id": null,
        "events": []
    }))
}

pub async fn pause_workflow(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let handle = {
        let executions = state.executions.read().expect("lock poisoned");
        executions.get(&id).cloned()
    };

    if let Some(handle) = handle {
        handle.runner_handle.pause().await;
        Json(json!({"status": "paused"}))
    } else {
        Json(json!({"error": "Workflow execution not found"}))
    }
}

pub async fn resume_workflow(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let handle = {
        let executions = state.executions.read().expect("lock poisoned");
        executions.get(&id).cloned()
    };

    if let Some(handle) = handle {
        handle.runner_handle.resume().await;
        Json(json!({"status": "resumed"}))
    } else {
        Json(json!({"error": "Workflow execution not found"}))
    }
}

pub async fn stop_workflow(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let handle = {
        let executions = state.executions.read().expect("lock poisoned");
        executions.get(&id).cloned()
    };

    if let Some(handle) = handle {
        handle.runner_handle.stop().await;
        Json(json!({ "status": "stopped" }))
    } else {
        Json(json!({ "error": "Workflow execution not found" }))
    }
}

pub async fn run_node(
    State(state): State<AppState>,
    Json(payload): Json<RunNodeRequest>,
) -> impl IntoResponse {
    let workspace_id = payload.space_id;
    let run_id = Uuid::new_v4().to_string();

    let (graph, start_input) = {
        let registry = &state.registry;

        let (graph, _) = match payload.blueprint.instantiate(registry) {
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

    // Prepare Runner
    let (mut runner, handle) = Runner::new(graph, None);

    // Setup bridge
    let tx = state.tx.clone();
    let run_id_clone = run_id.clone();
    let workspace_id_clone = workspace_id.clone();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    runner.set_event_sender(event_tx);

    let bridge_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let _ = tx.send((workspace_id_clone.clone(), run_id_clone.clone(), event));
        }
    });

    // Store execution handle
    {
        let mut executions = state.executions.write().expect("lock poisoned");
        executions.insert(
            run_id.clone(),
            ExecutionHandle {
                runner_handle: handle.clone(),
                workflow_id: -1,
                workspace_id: workspace_id.clone(),
            },
        );
    }

    let state_clone = state.clone();
    let start_node_id = payload.node_id.clone();
    let run_id_for_task = run_id.clone();
    let run_id_for_event = run_id.clone();
    let workspace_id_for_event = workspace_id.clone();

    tokio::spawn(async move {
        runner.set_start_node(&start_node_id, start_input.as_ref());

        if let Err(e) = runner.run().await {
            tracing::error!("Flow execution error: {}", e);
            let _ = state_clone.tx.send((
                workspace_id_for_event.clone(),
                run_id_for_event.clone(),
                FlowEvent::NodeError("runner".to_string(), e),
            ));
        }

        let _ = bridge_handle.await;

        {
            let mut executions = state_clone.executions.write().expect("lock poisoned");
            executions.remove(&run_id_for_task);
        }
    });

    Json(json!({"status": "started", "run_id": run_id, "start_node": payload.node_id}))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.workspace_id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, workspace_id: String) {
    let mut rx = state.tx.subscribe();
    while let Ok((msg_workspace_id, run_id, event)) = rx.recv().await {
        if msg_workspace_id == workspace_id {
            let wrapper = json!({
                "runId": run_id,
                "event": event
            });
            if let Ok(json) = serde_json::to_string(&wrapper) {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
