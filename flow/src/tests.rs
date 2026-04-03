use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

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
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
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
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
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
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
    runner.set_start_node("input", json!(42).into());
    runner
        .run()
        .await
        .expect("Conditional branching should succeed");
}
