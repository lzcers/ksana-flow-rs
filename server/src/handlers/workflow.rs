use crate::{
    registry,
    state::{AppState, ExecutionHandle, GraphBlueprint},
    utils,
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
    FlowEvent, NodeInputs, Runner, SendableAny, {ExecutionContext, NodeState},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::info;
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
    pub workflow_id: i64,
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

fn extract_single_output_value(outputs: &Value) -> Value {
    match outputs {
        Value::Object(map) => {
            if let Some(v) = map.get("output") {
                v.clone()
            } else if map.len() == 1 {
                map.values().next().cloned().unwrap_or(Value::Null)
            } else {
                outputs.clone()
            }
        }
        other => other.clone(),
    }
}

fn reconstruct_execution_context_from_blueprint(blueprint: &GraphBlueprint) -> ExecutionContext {
    let execution_ctx = ExecutionContext::new();
    for node in &blueprint.nodes {
        let node_status = if let Some(status) = &node.data.status {
            match status.as_str() {
                "completed" => NodeState::Completed,
                "failed" => NodeState::Failed,
                "running" => NodeState::Running,
                "skipped" => NodeState::Skipped,
                "idle" => NodeState::Idle,
                _ => NodeState::Pending,
            }
        } else {
            NodeState::Idle
        };

        // 节点状态恢复
        execution_ctx.set_state(node.id.clone(), node_status);
        // 节点输出恢复
        // 不一定完整，只能重建出能够序列化的输出
        let val = extract_single_output_value(&node.data.outputs);
        let output = restore_value(val);
        execution_ctx.set_output(node.id.clone(), output);
    }
    execution_ctx
}

async fn start_execution(
    state: AppState,
    graph: flow::Graph,
    init_execution_ctx: Option<ExecutionContext>,
    start_inputs: Vec<(String, NodeInputs)>,
    workflow_id: i64,
    workspace_id: String,
) -> Result<String, String> {
    let run_id = Uuid::new_v4().to_string();

    // Create execution in DB
    {
        let db = state.db.lock().expect("db lock poisoned");
        let _ = db.create_execution(&run_id, workflow_id);
    }

    // Prepare Runner
    let (mut runner, handle) = Runner::new(graph, init_execution_ctx);

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
            let _ = tx.send((
                workspace_id_clone.clone(),
                run_id_clone.clone(),
                utils::flow_event_to_value_lossy(&event),
            ));
        }
    });

    // Store execution handle
    {
        let mut executions = state.executions.write().expect("lock poisoned");
        executions.insert(
            run_id.clone(),
            ExecutionHandle {
                workflow_id,
                runner_handle: handle.clone(),
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
            runner.set_start_node_with_inputs(&node_id, input);
        }

        if let Err(e) = runner.run().await {
            tracing::error!("Flow execution error: {}", e);
            let event = crate::utils::flow_event_to_value_lossy(&FlowEvent::NodeError(
                "runner".to_string(),
                e,
            ));
            let _ = state_clone.tx.send((
                workspace_id_for_event.clone(),
                run_id_for_event.clone(),
                event,
            ));
        }

        let _ = bridge_handle.await;

        // Remove from executions
        {
            let mut executions = state_clone.executions.write().expect("lock poisoned");
            executions.remove(&run_id_for_task);
        }
    });

    Ok(run_id)
}

pub async fn run_workflow(
    State(state): State<AppState>,
    Json(request): Json<RunWorkflowRequest>,
) -> impl IntoResponse {
    let blueprint = request.blueprint;
    let workflow_id = request.workflow_id;
    let workspace_id = request.space_id;

    let (graph, start_inputs) = {
        let registry = &state.registry;

        // 解析 JSON 实例化整个蓝图
        let (graph, start_nodes) = match blueprint.instantiate(registry) {
            Ok(v) => v,
            Err(e) => return Json(json!({"error": e})),
        };

        let mut inputs = Vec::new();
        // 从蓝图中构建节点的初始输入
        for node_id in &start_nodes {
            if let Some(node) = blueprint.nodes.iter().find(|n| n.id == *node_id) {
                let default_val = if let Some(meta) = registry.get_node_metadata(&node.type_name) {
                    registry::create_default_value(&meta.inputs)
                } else {
                    Box::new(())
                };

                let mut map = HashMap::new();
                map.insert("external_start".to_owned(), default_val);
                inputs.push((node_id.clone(), NodeInputs::new(map)));
            }
        }
        (graph, inputs)
    };

    match start_execution(state, graph, None, start_inputs, workflow_id, workspace_id).await {
        Ok(run_id) => Json(json!({"status": "started", "run_id": run_id})),
        Err(e) => Json(json!({"error": e})),
    }
}

pub async fn run_node(
    State(state): State<AppState>,
    Json(payload): Json<RunNodeRequest>,
) -> impl IntoResponse {
    let workspace_id = payload.space_id;
    let workflow_id = payload.workflow_id;
    let blueprint = payload.blueprint;
    let node_id = payload.node_id;

    let (graph, execution_ctx, node_inputs) = {
        let registry = &state.registry;

        // 实例化蓝图
        let (graph, _) = match blueprint.instantiate(registry) {
            Ok(v) => v,
            Err(e) => return Json(json!({"error": e})),
        };

        // 执行上下文重建
        // Reconstruct ExecutionContext from blueprint
        let execution_ctx = reconstruct_execution_context_from_blueprint(&blueprint);

        // 重置当前节点状态
        execution_ctx.set_state(node_id.clone(), NodeState::Idle);

        // 提取所有父节点的输出构建当前节点输入
        let mut node_inputs = HashMap::new();
        for parent_id in graph.get_parents(&node_id) {
            let state = execution_ctx.get_state(&parent_id);
            match state {
                Some(NodeState::Completed) => {
                    if let Some(output) = execution_ctx.get_output(&parent_id) {
                        node_inputs.insert(parent_id.clone(), output);
                    }
                }
                _ => {
                    // 父节点未完成，理论上不应该允许从父节点未运行成功的地方开始执行
                    return Json(json!({
                        "error": format!("Parent node '{}' has not completed successfully", parent_id)
                    }));
                }
            }
        }
        (graph, execution_ctx, NodeInputs::new(node_inputs))
    };

    if !graph.nodes.contains_key(&node_id) {
        return Json(json!({
            "error": format!("Node '{}' not found in blueprint", node_id)
        }));
    }

    match start_execution(
        state,
        graph,
        Some(execution_ctx),
        vec![(node_id.clone(), node_inputs)],
        workflow_id,
        workspace_id,
    )
    .await
    {
        Ok(run_id) => Json(json!({
            "status": "started",
            "run_id": run_id,
            "start_node": node_id
        })),
        Err(e) => Json(json!({"error": e})),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        extract_single_output_value, reconstruct_execution_context_from_blueprint, restore_value,
    };
    use crate::state::{GraphBlueprint, Node, NodeData, Position};
    use flow::NodeState;
    use serde_json::{Value, json};

    #[test]
    fn extract_single_output_prefers_output_key() {
        let outputs = json!({"output": "x", "other": "y"});
        assert_eq!(extract_single_output_value(&outputs), json!("x"));
    }

    #[test]
    fn extract_single_output_uses_single_key_value() {
        let outputs = json!({"foo": 123});
        assert_eq!(extract_single_output_value(&outputs), json!(123));
    }

    #[test]
    fn extract_single_output_keeps_multi_key_object_without_output() {
        let outputs = json!({"a": 1, "b": 2});
        assert_eq!(extract_single_output_value(&outputs), outputs);
    }

    #[test]
    fn restore_value_string_becomes_string() {
        let payload = restore_value(json!("hello"));
        let s = payload
            .as_any()
            .downcast_ref::<String>()
            .cloned();
        assert_eq!(s, Some("hello".to_string()));
    }

    #[test]
    fn restore_value_null_becomes_unit() {
        let payload = restore_value(Value::Null);
        assert!(payload.as_any().downcast_ref::<()>().is_some());
    }

    #[test]
    fn restore_value_number_kept_as_json_value() {
        let payload = restore_value(json!(123));
        let v = payload
            .as_any()
            .downcast_ref::<Value>()
            .cloned();
        assert_eq!(v, Some(json!(123)));
    }

    #[test]
    fn reconstruct_execution_contextut() {
        let blueprint = GraphBlueprint {
            nodes: vec![
                Node {
                    id: "a".to_string(),
                    type_name: "TextNode".to_string(),
                    data: NodeData {
                        status: Some("completed".to_string()),
                        outputs: json!({"output": "cached-a"}),
                        ..Default::default()
                    },
                    position: Position { x: 0.0, y: 0.0 },
                    width: None,
                    height: None,
                },
                Node {
                    id: "b".to_string(),
                    type_name: "TextNode".to_string(),
                    data: NodeData {
                        status: Some("completed".to_string()),
                        outputs: json!({"output": "cached-b"}),
                        ..Default::default()
                    },
                    position: Position { x: 0.0, y: 0.0 },
                    width: None,
                    height: None,
                },
            ],
            edges: vec![],
        };

        let ctx = reconstruct_execution_context_from_blueprint(&blueprint);
        assert_eq!(ctx.get_state("a"), Some(NodeState::Completed));
        assert_eq!(ctx.get_state("b"), Some(NodeState::Completed));
        assert!(ctx.get_output("b").is_some());
        assert!(ctx.get_output("a").is_some());
    }
}
