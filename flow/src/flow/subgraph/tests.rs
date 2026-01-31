use super::*;
use async_trait::async_trait;
use crate::flow::{Context, Edge, Graph, Node, Output};
use crate::flow::event::FlowEvent;
use serde_json::json;
use tokio::sync::mpsc;

#[test]
fn test_subgraph_config_default() {
    let config = SubgraphConfig::default();
    assert_eq!(config.timeout, None);
    assert!(!config.inherit_context);
    assert_eq!(config.entry_node, "start");
    assert_eq!(config.exit_node, "end");
}

#[test]
fn test_subgraph_executor_creation() {
    let graph = Graph::new();
    let executor = SubgraphExecutor::with_defaults(graph);

    assert_eq!(executor.config().timeout, None);
    assert!(!executor.config().inherit_context);
    assert_eq!(executor.config().entry_node, "start");
    assert_eq!(executor.config().exit_node, "end");
}

#[test]
fn test_subgraph_error_display() {
    let error = SubgraphError::ExecutionFailed {
        message: "test error".to_string(),
    };
    assert_eq!(
        format!("{}", error),
        "Subgraph execution failed: test error"
    );

    let error = SubgraphError::Timeout {
        limit: std::time::Duration::from_secs(5),
    };
    assert!(
        format!("{}", error).contains("timeout")
    );
}

struct EchoNode;

#[async_trait]
impl Node for EchoNode {
    async fn run(&mut self, _ctx: &Context, input: &crate::flow::Input) -> Result<Output, String> {
        Ok(input.get_any().cloned().unwrap_or_default().into())
    }
}

#[tokio::test]
async fn test_subgraph_events_forwarded_via_context_sender() {
    let mut g = Graph::new();
    g.add_node("start", EchoNode);
    g.add_node("mid", EchoNode);
    g.add_node("end", EchoNode);
    g.add_edge(Edge {
        from: "start".to_string(),
        to: "mid".to_string(),
        condition: None,
    });
    g.add_edge(Edge {
        from: "mid".to_string(),
        to: "end".to_string(),
        condition: None,
    });

    let executor = SubgraphExecutor::with_defaults(g);

    let parent_ctx = Context::new();
    let (tx, mut rx) = mpsc::channel::<FlowEvent>(128);
    parent_ctx.set_any("__flow_event_sender", tx);

    let out = executor.execute(json!({"hello": "world"}), &parent_ctx).await.unwrap();
    assert_eq!(out, json!({"hello": "world"}));

    let mut saw_mid_started = false;
    let mut saw_flow_finished = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            FlowEvent::NodeStarted(id) if id == "mid" => {
                saw_mid_started = true;
            }
            FlowEvent::FlowFinished => {
                saw_flow_finished = true;
                break;
            }
            _ => {}
        }
    }

    assert!(saw_mid_started);
    assert!(saw_flow_finished);
}
