use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use crate::{
    Context, Controller, ControllerRunners, Edge, Graph, Input, Node, Output, RunnerKind,
    TriggerStrategy,
};

#[tokio::test]
async fn test_complex_graph_connections() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let s: String = input.get_any_as().unwrap_or_default();
            Ok(Value::String(s).into())
        }
    }

    struct BranchNode {
        branch_id: String,
    }
    impl BranchNode {
        fn new(id: &str) -> Self {
            Self {
                branch_id: id.to_string(),
            }
        }
    }
    #[async_trait]
    impl Node for BranchNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let s: String = input.get_any_as().unwrap_or_default();
            Ok(Value::String(format!("{}[{}]", s, self.branch_id)).into())
        }
    }

    struct MergeNode;
    #[async_trait]
    impl Node for MergeNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let mut entries: Vec<_> = input.get_values().iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            let joined = entries
                .iter()
                .filter_map(|(_, v)| v.as_str())
                .collect::<Vec<_>>()
                .join("");
            Ok(Value::String(format!("Merged: {}", joined)).into())
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("input", InputNode),
            ("branch1", BranchNode::new("A")),
            ("branch2", BranchNode::new("B")),
            ("branch3", BranchNode::new("C")),
            ("merge1", MergeNode),
            ("merge2", MergeNode),
        ],
        edges: [
            ("input", "branch1"),
            ("input", "branch2"),
            ("input", "branch3"),
            ("branch1", "merge1"),
            ("branch2", "merge1"),
            ("merge1", "merge2"),
            ("branch3", "merge2"),
        ]
    );

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None);
    runner.set_start_node("input", json!("Test").into());
    runner
        .run()
        .await
        .expect("Complex graph execution should succeed");
}

#[tokio::test]
async fn test_build_flow_macro_with_condition() {
    struct TestNode;
    #[async_trait]
    impl Node for TestNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let s: String = input.get_any_as().unwrap_or_default();
            Ok(Value::String(s).into())
        }
    }

    let is_true = |_ctx: &Context, _out: &String| true;

    let graph = crate::build_flow!(
        nodes: [
            ("node1", TestNode),
            ("node2", TestNode),
        ],
        edges: [
            ("node1", "node2", is_true),
        ]
    );

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None);
    runner.set_start_node("node1", json!("Start").into());
    runner.run().await.expect("Flow should succeed");
    runner.run().await.expect("Flow should succeed");
    runner.run().await.expect("Flow should succeed");
}

#[tokio::test]
async fn test_conditional_branching() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let n: i32 = input.get_any_as().unwrap_or_default();
            Ok(json!(n).into())
        }
    }

    struct CheckNode;
    #[async_trait]
    impl Node for CheckNode {
        const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AnyUpstreamAvailable;

        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let n: i32 = input.get_any_as().unwrap_or_default();
            Ok(serde_json::to_value((n, n > 0))
                .unwrap_or(Value::Null)
                .into())
        }
    }

    struct PositiveNode;
    #[async_trait]
    impl Node for PositiveNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let v: (i32, bool) = input.get_any_as().unwrap_or((0, false));
            Ok(Value::String(format!("Positive: {}", v.0)).into())
        }
    }

    struct NegativeNode;
    #[async_trait]
    impl Node for NegativeNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            let v: (i32, bool) = input.get_any_as().unwrap_or((0, false));
            Ok(Value::String(format!("Negative: {}", v.0)).into())
        }
    }

    let is_positive = |_ctx: &Context, output: &(i32, bool)| output.1;
    let is_negative = |_ctx: &Context, output: &(i32, bool)| !output.1;

    let graph = crate::build_flow!(
        nodes: [
            ("input", InputNode),
            ("check", CheckNode),
            ("positive", PositiveNode),
            ("negative", NegativeNode),
        ],
        edges: [
            ("input", "check"),
            ("check", "positive", is_positive),
            ("check", "negative", is_negative),
        ]
    );

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None);
    runner.set_start_node("input", json!(42).into());
    runner
        .run()
        .await
        .expect("Conditional branching should succeed");
}

#[tokio::test]
async fn test_runner_max_concurrency() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
            Ok(json!("start").into())
        }
    }

    struct SleepNode {
        in_flight: Arc<AtomicUsize>,
        max_in_flight: Arc<AtomicUsize>,
    }

    impl SleepNode {
        fn new(in_flight: Arc<AtomicUsize>, max_in_flight: Arc<AtomicUsize>) -> Self {
            Self {
                in_flight,
                max_in_flight,
            }
        }
    }

    #[async_trait]
    impl Node for SleepNode {
        async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
            let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            loop {
                let prev = self.max_in_flight.load(Ordering::SeqCst);
                if current <= prev {
                    break;
                }
                if self
                    .max_in_flight
                    .compare_exchange(prev, current, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(json!(current).into())
        }
    }

    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));

    let mut graph = Graph::new();
    graph.add_node("input", || InputNode);

    let in_flight_1 = in_flight.clone();
    let max_in_flight_1 = max_in_flight.clone();
    graph.add_node("sleep1", move || {
        SleepNode::new(in_flight_1.clone(), max_in_flight_1.clone())
    });

    let in_flight_2 = in_flight.clone();
    let max_in_flight_2 = max_in_flight.clone();
    graph.add_node("sleep2", move || {
        SleepNode::new(in_flight_2.clone(), max_in_flight_2.clone())
    });

    let in_flight_3 = in_flight.clone();
    let max_in_flight_3 = max_in_flight.clone();
    graph.add_node("sleep3", move || {
        SleepNode::new(in_flight_3.clone(), max_in_flight_3.clone())
    });

    let in_flight_4 = in_flight.clone();
    let max_in_flight_4 = max_in_flight.clone();
    graph.add_node("sleep4", move || {
        SleepNode::new(in_flight_4.clone(), max_in_flight_4.clone())
    });

    graph.add_edge(Edge {
        from: "input".to_string(),
        to: "sleep1".to_string(),
        condition: None,
    });
    graph.add_edge(Edge {
        from: "input".to_string(),
        to: "sleep2".to_string(),
        condition: None,
    });
    graph.add_edge(Edge {
        from: "input".to_string(),
        to: "sleep3".to_string(),
        condition: None,
    });
    graph.add_edge(Edge {
        from: "input".to_string(),
        to: "sleep4".to_string(),
        condition: None,
    });

    let (controller, _rx) = Controller::new();
    let (_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None);
    runner.set_max_concurrency(1);
    runner.set_start_node("input", json!(null).into());
    runner.run().await.expect("Flow should succeed");

    let observed = max_in_flight.load(Ordering::SeqCst);
    assert!(observed <= 1, "max concurrency exceeded: {}", observed);
}
