use crate::state::AppState;
use axum::{
    Json,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use flow::{AnyEdge, Edge, FlowEvent, Graph, Runner};
use nodes::trade::K;
use nodes::trade::backtester::BacktesterInput;
use serde_json::json;

pub async fn run_flow(State(state): State<AppState>) -> impl IntoResponse {
    if state.is_running() {
        return Json(json!({"error": "Workflow is already running"}));
    }

    let (graph, start_nodes) = {
        let blueprint = state
            .blueprint
            .read()
            .expect("Failed to read blueprint: lock poisoned");
        let registry = state
            .registry
            .read()
            .expect("Failed to read registry: lock poisoned");

        let mut new_graph = Graph::new();
        let mut start_nodes = Vec::new();

        for node_cfg in &blueprint.nodes {
            if let Ok(node) = registry.create_node(&node_cfg.type_name, node_cfg.config.clone()) {
                new_graph.add_arc_node(&node_cfg.id, node);
                if node_cfg.type_name == "ReactiveSourceNode" {
                    start_nodes.push(node_cfg.id.clone());
                }
            }
        }

        for edge_cfg in &blueprint.edges {
            let source_node = blueprint.nodes.iter().find(|n| n.id == edge_cfg.source);
            if let Some(source_node) = source_node {
                let type_name = &source_node.type_name;
                let edge: Option<Box<dyn AnyEdge>> = match type_name.as_str() {
                    "ReactiveSourceNode" => Some(Box::new(Edge::<K> {
                        from: edge_cfg.source.clone(),
                        to: edge_cfg.target.clone(),
                        condition: None,
                    })),
                    "VOLMFINode" => Some(Box::new(Edge::<BacktesterInput> {
                        from: edge_cfg.source.clone(),
                        to: edge_cfg.target.clone(),
                        condition: None,
                    })),
                    "Backtester" => Some(Box::new(Edge::<()> {
                        from: edge_cfg.source.clone(),
                        to: edge_cfg.target.clone(),
                        condition: None,
                    })),
                    _ => None,
                };
                if let Some(e) = edge {
                    new_graph
                        .edges
                        .entry(edge_cfg.source.clone())
                        .or_insert_with(Vec::new)
                        .push(e);
                }
            }
        }
        (new_graph, start_nodes)
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

            for node_id in start_nodes {
                runner = runner.set_start_node(&node_id, &());
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
