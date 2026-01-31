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

use crate::{Context, Input, Node, Output, Runner, TriggerStrategy};

fn input_with_external_start(value: Value) -> Input {
    let mut map = HashMap::new();
    map.insert("external_start".to_string(), value);
    Input::new(map)
}

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

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("input", json!("Test"));
    runner.run().await.expect("Complex graph execution should succeed");
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

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("node1", json!("Start"));
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
            Ok(serde_json::to_value((n, n > 0)).unwrap_or(Value::Null).into())
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

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node_with_inputs("input", input_with_external_start(json!(42)));
    runner.run().await.expect("Conditional branching should succeed");
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

    let graph = crate::build_flow!(
        nodes: [
            ("input", InputNode),
            ("sleep1", SleepNode::new(in_flight.clone(), max_in_flight.clone())),
            ("sleep2", SleepNode::new(in_flight.clone(), max_in_flight.clone())),
            ("sleep3", SleepNode::new(in_flight.clone(), max_in_flight.clone())),
            ("sleep4", SleepNode::new(in_flight.clone(), max_in_flight.clone())),
        ],
        edges: [
            ("input", "sleep1"),
            ("input", "sleep2"),
            ("input", "sleep3"),
            ("input", "sleep4"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_max_concurrency(1);
    runner.set_start_node_with_inputs("input", input_with_external_start(json!(null)));
    runner.run().await.expect("Flow should succeed");

    let observed = max_in_flight.load(Ordering::SeqCst);
    assert!(observed <= 1, "max concurrency exceeded: {}", observed);
}
