use super::{
    BlueprintEdge, BlueprintNode, SubgraphConfig, SubgraphError, SubgraphExecutor, compile_graph,
    graph::{AnyNode, Edge, Graph, Node, NodeFactory},
    io::{Input, Output},
};
use crate::{
    Context, Controller, ControllerRunners, RunnerKind,
    flow::{Runner, runner::FlowEvent},
    scope_controller,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Instant;
use tokio::sync::RwLock;

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
    assert!(format!("{}", error).contains("timeout"));
}

struct EchoNode;

#[async_trait]
impl Node for EchoNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        Ok(input.get_any().cloned().unwrap_or_default().into())
    }
}

struct StartNode;

#[async_trait]
impl Node for StartNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        Ok(json!({"ok": true}).into())
    }
}

struct ConstNode {
    value: Value,
}

#[async_trait]
impl Node for ConstNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        Ok(self.value.clone().into())
    }
}

struct CountNode {
    counter: Arc<AtomicUsize>,
}

#[async_trait]
impl Node for CountNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null.into())
    }
}

struct SleepNode;

#[async_trait]
impl Node for SleepNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        Ok(Value::Null.into())
    }
}

#[tokio::test]
async fn test_compile_graph_bool_condition_blocks_edge() {
    let counter = Arc::new(AtomicUsize::new(0));

    let nodes = vec![
        BlueprintNode {
            id: "A".to_string(),
            type_name: "StartNode".to_string(),
            parent_id: None,
            config: Value::Null,
        },
        BlueprintNode {
            id: "B".to_string(),
            type_name: "CountNode".to_string(),
            parent_id: None,
            config: Value::Null,
        },
    ];

    let edges = vec![BlueprintEdge {
        id: "e1".to_string(),
        source: "A".to_string(),
        target: "B".to_string(),
        source_handle: None,
        target_handle: None,
        condition: Some(Value::Bool(false)),
    }];

    let counter_for_factory = counter.clone();
    let create_leaf_factory = move |node: &BlueprintNode| -> Result<Arc<NodeFactory>, String> {
        match node.type_name.as_str() {
            "StartNode" => Ok(Arc::new(move || {
                Ok(Arc::new(RwLock::new(StartNode)) as Arc<RwLock<dyn AnyNode>>)
            })),
            "CountNode" => {
                let counter = counter_for_factory.clone();
                Ok(Arc::new(move || {
                    Ok(Arc::new(RwLock::new(CountNode {
                        counter: counter.clone(),
                    })) as Arc<RwLock<dyn AnyNode>>)
                }))
            }
            other => Err(format!("unknown node type: {}", other)),
        }
    };

    let (graph, start_nodes) =
        compile_graph(&nodes, &edges, "SubgraphNode", create_leaf_factory).unwrap();

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(graph, None, RunnerKind::Root, None, None);
    for id in start_nodes {
        runner.set_start_node(&id, Value::Null.into());
    }
    runner.run().await.unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_compile_graph_filters_dangling_edges() {
    let counter = Arc::new(AtomicUsize::new(0));

    let nodes = vec![BlueprintNode {
        id: "B".to_string(),
        type_name: "CountNode".to_string(),
        parent_id: None,
        config: Value::Null,
    }];

    let edges = vec![BlueprintEdge {
        id: "e1".to_string(),
        source: "Missing".to_string(),
        target: "B".to_string(),
        source_handle: None,
        target_handle: None,
        condition: None,
    }];

    let counter_for_factory = counter.clone();
    let create_leaf_factory = move |node: &BlueprintNode| -> Result<Arc<NodeFactory>, String> {
        match node.type_name.as_str() {
            "CountNode" => {
                let counter = counter_for_factory.clone();
                Ok(Arc::new(move || {
                    Ok(Arc::new(RwLock::new(CountNode {
                        counter: counter.clone(),
                    })) as Arc<RwLock<dyn AnyNode>>)
                }))
            }
            other => Err(format!("unknown node type: {}", other)),
        }
    };

    let (graph, start_nodes) =
        compile_graph(&nodes, &edges, "SubgraphNode", create_leaf_factory).unwrap();

    assert_eq!(start_nodes, vec!["B".to_string()]);
    assert!(graph.incoming_nodes.get("B").is_none());
    assert!(graph.edges.get("Missing").is_none());

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(graph, None, RunnerKind::Root, None, None);
    for id in start_nodes {
        runner.set_start_node(&id, Value::Null.into());
    }
    runner.run().await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_subgraph_events_forwarded_via_controller() {
    let mut g = Graph::new();
    g.add_node("start", || EchoNode);
    g.add_node("mid", || EchoNode);
    g.add_node("end", || EchoNode);
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
    let (controller, mut rx) = Controller::new();

    let out = scope_controller(controller, async {
        executor
            .execute(json!({"hello": "world"}), &parent_ctx)
            .await
    })
    .await
    .unwrap();
    assert_eq!(out, json!({"hello": "world"}));

    let mut saw_mid_started = false;
    let mut saw_flow_finished = false;
    while let Some(ev) = rx.recv().await {
        match ev.event {
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

#[tokio::test]
async fn test_subgraph_inbound_proxy_routes_by_source_id() {
    let nodes = vec![
        BlueprintNode {
            id: "A".to_string(),
            type_name: "ConstNode".to_string(),
            parent_id: None,
            config: json!("va"),
        },
        BlueprintNode {
            id: "B".to_string(),
            type_name: "ConstNode".to_string(),
            parent_id: None,
            config: json!("vb"),
        },
        BlueprintNode {
            id: "G".to_string(),
            type_name: "SubgraphNode".to_string(),
            parent_id: None,
            config: Value::Null,
        },
        BlueprintNode {
            id: "X".to_string(),
            type_name: "EchoNode".to_string(),
            parent_id: Some("G".to_string()),
            config: Value::Null,
        },
        BlueprintNode {
            id: "Y".to_string(),
            type_name: "EchoNode".to_string(),
            parent_id: Some("G".to_string()),
            config: Value::Null,
        },
    ];

    let edges = vec![
        BlueprintEdge {
            id: "e_AX".to_string(),
            source: "A".to_string(),
            target: "X".to_string(),
            source_handle: None,
            target_handle: None,
            condition: None,
        },
        BlueprintEdge {
            id: "e_BY".to_string(),
            source: "B".to_string(),
            target: "Y".to_string(),
            source_handle: None,
            target_handle: None,
            condition: None,
        },
    ];

    let create_leaf_factory = move |node: &BlueprintNode| -> Result<Arc<NodeFactory>, String> {
        match node.type_name.as_str() {
            "ConstNode" => {
                let value = node.config.clone();
                Ok(Arc::new(move || {
                    Ok(Arc::new(RwLock::new(ConstNode {
                        value: value.clone(),
                    })) as Arc<RwLock<dyn AnyNode>>)
                }))
            }
            "EchoNode" => Ok(Arc::new(move || {
                Ok(Arc::new(RwLock::new(EchoNode)) as Arc<RwLock<dyn AnyNode>>)
            })),
            other => Err(format!("unknown node type: {}", other)),
        }
    };

    let (graph, start_nodes) =
        compile_graph(&nodes, &edges, "SubgraphNode", create_leaf_factory).unwrap();

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(graph, None, RunnerKind::Root, None, None);
    for id in start_nodes {
        runner.set_start_node(&id, Value::Null.into());
    }
    runner.run().await.unwrap();

    let out = runner.get_execution_context().get_output("G").unwrap();
    assert_eq!(out, json!({"X": "va", "Y": "vb"}));
}

#[tokio::test]
async fn test_subgraph_inbound_proxy_single_source_passthrough() {
    let nodes = vec![
        BlueprintNode {
            id: "A".to_string(),
            type_name: "ConstNode".to_string(),
            parent_id: None,
            config: json!("va"),
        },
        BlueprintNode {
            id: "G".to_string(),
            type_name: "SubgraphNode".to_string(),
            parent_id: None,
            config: Value::Null,
        },
        BlueprintNode {
            id: "X".to_string(),
            type_name: "EchoNode".to_string(),
            parent_id: Some("G".to_string()),
            config: Value::Null,
        },
        BlueprintNode {
            id: "Y".to_string(),
            type_name: "EchoNode".to_string(),
            parent_id: Some("G".to_string()),
            config: Value::Null,
        },
    ];

    let edges = vec![
        BlueprintEdge {
            id: "e_AX".to_string(),
            source: "A".to_string(),
            target: "X".to_string(),
            source_handle: None,
            target_handle: None,
            condition: None,
        },
        BlueprintEdge {
            id: "e_AY".to_string(),
            source: "A".to_string(),
            target: "Y".to_string(),
            source_handle: None,
            target_handle: None,
            condition: None,
        },
    ];

    let create_leaf_factory = move |node: &BlueprintNode| -> Result<Arc<NodeFactory>, String> {
        match node.type_name.as_str() {
            "ConstNode" => {
                let value = node.config.clone();
                Ok(Arc::new(move || {
                    Ok(Arc::new(RwLock::new(ConstNode {
                        value: value.clone(),
                    })) as Arc<RwLock<dyn AnyNode>>)
                }))
            }
            "EchoNode" => Ok(Arc::new(move || {
                Ok(Arc::new(RwLock::new(EchoNode)) as Arc<RwLock<dyn AnyNode>>)
            })),
            other => Err(format!("unknown node type: {}", other)),
        }
    };

    let (graph, start_nodes) =
        compile_graph(&nodes, &edges, "SubgraphNode", create_leaf_factory).unwrap();

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(graph, None, RunnerKind::Root, None, None);
    for id in start_nodes {
        runner.set_start_node(&id, Value::Null.into());
    }
    runner.run().await.unwrap();

    let out = runner.get_execution_context().get_output("G").unwrap();
    assert_eq!(out, json!({"X": "va", "Y": "va"}));
}

#[tokio::test]
async fn test_stop_emits_flow_stopped() {
    let mut g = Graph::new();
    g.add_node("start", || SleepNode);

    let graph = Arc::new(g);
    let (controller, mut rx) = Controller::new();
    let (runner_id, mut runner, handle) =
        controller.create_runner(graph, None, RunnerKind::Root, None, None);
    runner.set_start_node("start", Value::Null.into());

    let join = controller.spawn_runner(runner_id, runner);

    handle.stop().await;

    let mut saw_stopped = false;
    for _ in 0..40 {
        if let Ok(Some(event)) =
            tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
        {
            if matches!(event.event, FlowEvent::FlowStopped) {
                saw_stopped = true;
                break;
            }
        }
    }

    assert!(saw_stopped);
    let _ = join.await;
}

#[test]
fn bench_compile_graph_200_nodes() {
    let nodes: Vec<BlueprintNode> = (0..200)
        .map(|i| BlueprintNode {
            id: format!("N{}", i),
            type_name: "StartNode".to_string(),
            parent_id: None,
            config: Value::Null,
        })
        .collect();

    let edges: Vec<BlueprintEdge> = (0..199)
        .map(|i| BlueprintEdge {
            id: format!("e{}", i),
            source: format!("N{}", i),
            target: format!("N{}", i + 1),
            source_handle: None,
            target_handle: None,
            condition: None,
        })
        .collect();

    let create_leaf_factory = |_node: &BlueprintNode| -> Result<Arc<NodeFactory>, String> {
        Ok(Arc::new(move || {
            Ok(Arc::new(RwLock::new(StartNode)) as Arc<RwLock<dyn AnyNode>>)
        }))
    };

    let start = Instant::now();
    for _ in 0..100 {
        let _ = compile_graph(&nodes, &edges, "SubgraphNode", create_leaf_factory.clone()).unwrap();
    }
    let _elapsed = start.elapsed();
}
