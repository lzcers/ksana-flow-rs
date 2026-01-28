use crate::flow::{Context, Node, NodeInputs};
use crate::flow::{GraphBuilder, Runner};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::OutputPayload;

fn get_input<T: 'static + Clone>(inputs: &NodeInputs) -> T {
    for payload in inputs.inputs.values() {
        let Some(any) = payload.as_any() else {
            continue;
        };
        if let Some(v) = any.downcast_ref::<T>() {
            return v.clone();
        }
    }
    panic!("Expected input of type {}", std::any::type_name::<T>())
}

#[tokio::test]
async fn test_complex_graph_connections() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
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
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("{}[{}]", input, self.branch_id))
        }
    }

    struct MergeNode;
    #[async_trait]
    impl Node for MergeNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("Merged: {}", input))
        }
    }

    struct OutputNode;
    #[async_trait]
    impl Node for OutputNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
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
            ("output", OutputNode),
        ],
        edges: [
            ("input", "branch1"),
            ("input", "branch2"),
            ("input", "branch3"),
            ("branch1", "merge1"),
            ("branch2", "merge1"),
            ("merge1", "merge2"),
            ("branch3", "merge2"),
            ("merge2", "output"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("input", OutputPayload::cloned("Test".to_string()));
    runner
        .run()
        .await
        .expect("Complex graph execution should succeed");
}

#[tokio::test]
async fn test_build_flow_macro() {
    struct TestNode;
    #[async_trait]
    impl Node for TestNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("node1", TestNode),
            ("node2", TestNode),
            ("node3", TestNode),
        ],
        edges: [
            ("node1", "node2"),
            ("node2", "node3"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("node1", OutputPayload::cloned("Start".to_string()));
    let result = runner.run().await;

    assert!(result.is_ok());

    let is_true = |_ctx: &Context, _out: &String| true;
    let graph_cond = crate::build_flow!(
        nodes: [
            ("node1", TestNode),
            ("node2", TestNode),
        ],
        edges: [
            ("node1", "node2", is_true),
        ]
    );
    let (mut runner_cond, _handle) = Runner::new(graph_cond, None);
    runner_cond.set_start_node("node1", OutputPayload::cloned("Start".to_string()));
    let result_cond = runner_cond.run().await;
    assert!(result_cond.is_ok());
}

#[tokio::test]
async fn test_conditional_branching() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<i32>(&inputs))
        }
    }

    struct CheckNode;
    #[async_trait]
    impl Node for CheckNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<i32>(&inputs);
            OutputPayload::cloned((input, input > 0))
        }
    }

    struct PositiveNode;
    #[async_trait]
    impl Node for PositiveNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<(i32, bool)>(&inputs);
            OutputPayload::cloned(format!("Positive: {}", input.0))
        }
    }

    struct NegativeNode;
    #[async_trait]
    impl Node for NegativeNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<(i32, bool)>(&inputs);
            OutputPayload::cloned(format!("Negative: {}", input.0))
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
    runner.set_start_node("input", OutputPayload::cloned(42));
    let result = runner.run().await;

    assert!(result.is_ok(), "Conditional branching should succeed");
}

#[tokio::test]
async fn test_multi_level_graph() {
    struct LevelNode {
        level: u32,
    }
    impl LevelNode {
        fn new(level: u32) -> Self {
            Self { level }
        }
    }
    #[async_trait]
    impl Node for LevelNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("L{}: {}", self.level, input))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("l1_1", LevelNode::new(1)),
            ("l1_2", LevelNode::new(1)),
            ("l2_1", LevelNode::new(2)),
            ("l2_2", LevelNode::new(2)),
            ("l2_3", LevelNode::new(2)),
            ("l3_1", LevelNode::new(3)),
            ("l3_2", LevelNode::new(3)),
            ("l4_1", LevelNode::new(4)),
        ],
        edges: [
            ("l1_1", "l2_1"),
            ("l1_1", "l2_2"),
            ("l1_2", "l2_2"),
            ("l1_2", "l2_3"),
            ("l2_1", "l3_1"),
            ("l2_2", "l3_1"),
            ("l2_2", "l3_2"),
            ("l2_3", "l3_2"),
            ("l3_1", "l4_1"),
            ("l3_2", "l4_1"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("l1_1", OutputPayload::cloned("Start".to_string()));
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "Multi-level graph should execute successfully"
    );
}

#[tokio::test]
async fn test_large_concurrent_nodes() {
    struct CounterNode {
        id: usize,
    }
    impl CounterNode {
        fn new(id: usize) -> Self {
            Self { id }
        }
    }
    #[async_trait]
    impl Node for CounterNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("{}[Node{}]", input, self.id))
        }
    }

    struct AggregatorNode;
    #[async_trait]
    impl Node for AggregatorNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("Aggregated: {}", input))
        }
    }

    let mut builder = GraphBuilder::new().add_node("input", CounterNode::new(0));

    for i in 1..=50 {
        builder = builder.add_node(format!("node{}", i).as_str(), CounterNode::new(i));
        builder = builder.add_edge("input", format!("node{}", i).as_str());
    }

    builder = builder.add_node("aggregator", AggregatorNode);

    for i in 1..=50 {
        builder = builder.add_edge(format!("node{}", i).as_str(), "aggregator");
    }

    let graph = builder.build();
    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("input", OutputPayload::cloned("Data".to_string()));
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "Large concurrent nodes should execute successfully"
    );
}

#[tokio::test]
async fn test_concurrent_execution_tracking() {
    struct TrackingNode {
        id: usize,
        execution_log: Arc<Mutex<Vec<usize>>>,
    }
    impl TrackingNode {
        fn new(id: usize, log: Arc<Mutex<Vec<usize>>>) -> Self {
            Self {
                id,
                execution_log: log,
            }
        }
    }
    #[async_trait]
    impl Node for TrackingNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            {
                let mut log = self.execution_log.lock().unwrap();
                log.push(self.id);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
            OutputPayload::cloned(format!("{}[{}]", input, self.id))
        }
    }

    struct CollectorNode;
    #[async_trait]
    impl Node for CollectorNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
        }
    }

    let execution_log = Arc::new(Mutex::new(Vec::new()));

    let mut builder =
        GraphBuilder::new().add_node("input", TrackingNode::new(0, execution_log.clone()));

    for i in 1..=20 {
        builder = builder.add_node(
            format!("node{}", i).as_str(),
            TrackingNode::new(i, execution_log.clone()),
        );
        builder = builder.add_edge("input", format!("node{}", i).as_str());
    }

    builder = builder.add_node("collector", CollectorNode);

    for i in 1..=20 {
        builder = builder.add_edge(format!("node{}", i).as_str(), "collector");
    }

    let graph = builder.build();
    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("input", OutputPayload::cloned("Concurrent".to_string()));
    let result = runner.run().await;

    assert!(result.is_ok(), "Concurrent execution should succeed");

    let log = execution_log.lock().unwrap();
    assert_eq!(log.len(), 21, "All nodes should have executed");
}

#[tokio::test]
async fn test_context_write_and_read() {
    struct WriterNode;
    #[async_trait]
    impl Node for WriterNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            ctx.set("key1", "value1");
            ctx.set("key2", 42);
            ctx.set("key3", vec![1, 2, 3]);
            OutputPayload::cloned(input)
        }
    }

    struct ReaderNode;
    #[async_trait]
    impl Node for ReaderNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            let value1: Option<String> = ctx.get("key1");
            let value2: Option<i32> = ctx.get("key2");
            let value3: Option<Vec<i32>> = ctx.get("key3");

            assert_eq!(value1, Some("value1".to_string()));
            assert_eq!(value2, Some(42));
            assert_eq!(value3, Some(vec![1, 2, 3]));

            OutputPayload::cloned(format!("Read: {}", input))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("writer", WriterNode),
            ("reader", ReaderNode),
        ],
        edges: [
            ("writer", "reader"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("writer", OutputPayload::cloned("Test".to_string()));
    let result = runner.run().await;

    assert!(result.is_ok(), "Context read/write should succeed");
}

#[tokio::test]
async fn test_context_across_multiple_nodes() {
    struct Node1;
    #[async_trait]
    impl Node for Node1 {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            ctx.set("node1_data", "from_node1");
            ctx.set("counter", 1);
            OutputPayload::cloned(input)
        }
    }

    struct Node2;
    #[async_trait]
    impl Node for Node2 {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            let counter: Option<i32> = ctx.get("counter");
            if let Some(c) = counter {
                ctx.set("counter", c + 1);
            }
            ctx.set("node2_data", "from_node2");
            OutputPayload::cloned(input)
        }
    }

    struct Node3;
    #[async_trait]
    impl Node for Node3 {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            let node1_data: Option<String> = ctx.get("node1_data");
            let node2_data: Option<String> = ctx.get("node2_data");
            let counter: Option<i32> = ctx.get("counter");

            assert_eq!(node1_data, Some("from_node1".to_string()));
            assert_eq!(node2_data, Some("from_node2".to_string()));
            assert_eq!(counter, Some(2));

            OutputPayload::cloned(format!("Final: {}", input))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("node1", Node1),
            ("node2", Node2),
            ("node3", Node3),
        ],
        edges: [
            ("node1", "node2"),
            ("node2", "node3"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("node1", OutputPayload::cloned("Start".to_string()));
    let result = runner.run().await;

    assert!(result.is_ok(), "Context across multiple nodes should work");
}

#[tokio::test]
async fn test_context_with_conditional_edges() {
    struct InputNode;
    #[async_trait]
    impl Node for InputNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<i32>(&inputs);
            ctx.set("input_value", input);
            OutputPayload::cloned(input)
        }
    }

    struct CheckNode;
    #[async_trait]
    impl Node for CheckNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<i32>(&inputs);
            let stored: Option<i32> = ctx.get("input_value");
            assert_eq!(stored, Some(input));
            OutputPayload::cloned((input, input % 2 == 0))
        }
    }

    struct EvenNode;
    #[async_trait]
    impl Node for EvenNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<(i32, bool)>(&inputs);
            ctx.set("result", "even");
            OutputPayload::cloned(format!("Even: {}", input.0))
        }
    }

    struct OddNode;
    #[async_trait]
    impl Node for OddNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<(i32, bool)>(&inputs);
            ctx.set("result", "odd");
            OutputPayload::cloned(format!("Odd: {}", input.0))
        }
    }

    let is_even = |_ctx: &Context, output: &(i32, bool)| output.1;
    let is_odd = |_ctx: &Context, output: &(i32, bool)| !output.1;

    let graph = crate::build_flow!(
        nodes: [
            ("input", InputNode),
            ("check", CheckNode),
            ("even", EvenNode),
            ("odd", OddNode),
        ],
        edges: [
            ("input", "check"),
            ("check", "even", is_even),
            ("check", "odd", is_odd),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("input", OutputPayload::cloned(4));
    let result = runner.run().await;

    assert!(result.is_ok(), "Context with conditional edges should work");
}

#[tokio::test]
async fn test_context_serialization() {
    struct SerializeNode;
    #[async_trait]
    impl Node for SerializeNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            ctx.set("string", "hello");
            ctx.set("number", 123);
            ctx.set("float", 45.67);
            ctx.set("boolean", true);
            ctx.set("array", vec![1, 2, 3, 4, 5]);
            OutputPayload::cloned(input)
        }
    }

    struct DeserializeNode;
    #[async_trait]
    impl Node for DeserializeNode {
        async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            let s: Option<String> = ctx.get("string");
            let n: Option<i32> = ctx.get("number");
            let f: Option<f64> = ctx.get("float");
            let b: Option<bool> = ctx.get("boolean");
            let a: Option<Vec<i32>> = ctx.get("array");

            assert_eq!(s, Some("hello".to_string()));
            assert_eq!(n, Some(123));
            assert_eq!(f, Some(45.67));
            assert_eq!(b, Some(true));
            assert_eq!(a, Some(vec![1, 2, 3, 4, 5]));

            OutputPayload::cloned(format!("Deserialized: {}", input))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("serialize", SerializeNode),
            ("deserialize", DeserializeNode),
        ],
        edges: [
            ("serialize", "deserialize"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("serialize", OutputPayload::cloned("Test".to_string()));
    let result = runner.run().await;

    assert!(result.is_ok(), "Context serialization should work");
}

#[tokio::test]
async fn test_diamond_graph_pattern() {
    struct StartNode;
    #[async_trait]
    impl Node for StartNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
        }
    }

    struct LeftNode;
    #[async_trait]
    impl Node for LeftNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("L: {}", input))
        }
    }

    struct RightNode;
    #[async_trait]
    impl Node for RightNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("R: {}", input))
        }
    }

    struct EndNode;
    #[async_trait]
    impl Node for EndNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let input = get_input::<String>(&inputs);
            OutputPayload::cloned(format!("E: {}", input))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("start", StartNode),
            ("left", LeftNode),
            ("right", RightNode),
            ("end", EndNode),
        ],
        edges: [
            ("start", "left"),
            ("start", "right"),
            ("left", "end"),
            ("right", "end"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("start", OutputPayload::cloned("Diamond".to_string()));
    runner
        .run()
        .await
        .expect("Diamond pattern should execute successfully");
}

#[tokio::test]
async fn test_type_mismatch() {
    struct StringNode;
    #[async_trait]
    impl Node for StringNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
        }
    }

    struct IntNode;
    #[async_trait]
    impl Node for IntNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<i32>(&inputs))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("string_node", StringNode),
            ("int_node", IntNode),
        ],
        edges: [
            ("string_node", "int_node"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("string_node", OutputPayload::cloned("Test".to_string()));
    let result = runner.run().await;

    assert!(result.is_err(), "Type mismatch should cause error");
    assert!(result.unwrap_err().contains("Expected input of type"));
}

#[tokio::test]
async fn test_unit_input_allows_any() {
    struct StringNode;
    #[async_trait]
    impl Node for StringNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(get_input::<String>(&inputs))
        }
    }

    struct UnitNode;
    #[async_trait]
    impl Node for UnitNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned("UnitNode called".to_string())
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("string_node", StringNode),
            ("unit_node", UnitNode),
        ],
        edges: [
            ("string_node", "unit_node"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("string_node", OutputPayload::cloned("Test".to_string()));
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "Unit input node should allow any input type, got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_no_start_nodes_hang_fix() {
    struct TestNode;
    #[async_trait]
    impl Node for TestNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(())
        }
    }

    let graph = crate::build_flow!(
        nodes: [("node1", TestNode)],
        edges: []
    );

    // Create runner but do NOT set start node
    let (mut runner, _handle) = Runner::new(graph, None);

    // Use timeout to ensure it doesn't hang
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), runner.run()).await;

    assert!(
        result.is_ok(),
        "Runner should finish immediately and not hang"
    );
    assert!(
        result.unwrap().is_ok(),
        "Runner execution should be successful"
    );
}

#[tokio::test]
async fn test_multi_input_merge() {
    use crate::flow::NodeInputs;
    struct SourceNode {
        value: String,
    }
    #[async_trait]
    impl Node for SourceNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            OutputPayload::cloned(self.value.clone())
        }
    }

    struct MultiMergeNode;
    #[async_trait]
    impl Node for MultiMergeNode {
        async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> OutputPayload {
            let mut parts: Vec<String> = inputs
                .inputs
                .values()
                .filter_map(|p| p.as_any().and_then(|a| a.downcast_ref::<String>()).cloned())
                .collect();
            parts.sort(); // Ensure deterministic order
            OutputPayload::cloned(format!("Merged: {}", parts.join(", ")))
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("source1", SourceNode { value: "A".to_string() }),
            ("source2", SourceNode { value: "B".to_string() }),
            ("merge", MultiMergeNode),
        ],
        edges: [
            ("source1", "merge"),
            ("source2", "merge"),
        ]
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    // Trigger source1 and source2
    // Since we don't have a common start node, we can trigger them individually or use a start node.
    // Let's use a dummy start node.

    // Actually, set_start_node puts tasks in queue.
    runner.set_start_node("source1", OutputPayload::cloned(()));
    runner.set_start_node("source2", OutputPayload::cloned(()));

    let result = runner.run().await;
    assert!(result.is_ok());
    // We can't easily assert output here without capturing it, but execution success means it ran.
    // To verify output, we could use a context or something, but `flatten_sendable_any` print might show it if we enable logs.
    // Or we can add an assertion inside the node? No, let's use a side effect.
}

#[tokio::test]
async fn test_stream_node_emits_completed() {
    use crate::flow::{FlowEvent, ReactiveStream};

    struct StreamNode;
    #[async_trait]
    impl Node for StreamNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            let data = vec!["A".to_string()];
            let stream = ReactiveStream::from_observable(data);
            OutputPayload::stream(stream.subscribe)
        }
    }

    let graph = crate::build_flow!(
        nodes: [("stream", StreamNode)],
        edges: []
    );

    let (mut runner, _handle) = Runner::new(graph, None);
    runner.set_start_node("stream", OutputPayload::cloned("start".to_string()));

    let (tx, mut rx) = tokio::sync::mpsc::channel(20);
    runner.set_event_sender(tx);

    let runner_task = tokio::spawn(async move { runner.run().await });

    let mut completed_received = false;
    let mut next_received = false;

    while let Some(event) = rx.recv().await {
        match event {
            FlowEvent::NodeCompleted(id) if id == "stream" => {
                completed_received = true;
            }
            FlowEvent::NodeStreamNextMessage(id, _) if id == "stream" => {
                next_received = true;
            }
            FlowEvent::FlowFinished => break,
            _ => {}
        }
    }

    runner_task.await.unwrap().unwrap();

    assert!(next_received, "Should receive stream message");
    assert!(completed_received, "Should receive NodeCompleted event");
}

#[tokio::test]
async fn test_runner_keeps_stream_subscription_alive() {
    use crate::flow::{FlowEvent, TaskEvent};
    use crate::{Context, NodeInputs, NodeId, OutputPayload, StreamSubscriptionFn, TaskGuard};
    use crate::observable::Subscription;
    use tokio::sync::mpsc;

    struct AbortOnDropSubscription {
        abort: tokio::task::AbortHandle,
    }

    impl Drop for AbortOnDropSubscription {
        fn drop(&mut self) {
            self.abort.abort();
        }
    }

    impl Subscription for AbortOnDropSubscription {
        fn unsubscribe(self) {
            self.abort.abort();
        }
    }

    struct CancelStreamNode;
    #[async_trait]
    impl crate::Node for CancelStreamNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            let subscribe: StreamSubscriptionFn =
                Box::new(move |guard: TaskGuard, tx: mpsc::Sender<TaskEvent>, node_id: NodeId, _| {
                    let handle = tokio::spawn(async move {
                        let _guard = guard;
                        let _ = tx
                            .send(TaskEvent::Next(node_id.clone(), OutputPayload::cloned("chunk".to_string())))
                            .await;
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                        let _ = tx.send(TaskEvent::Completed(node_id, None)).await;
                    });
                    let abort = handle.abort_handle();
                    drop(handle);
                    Box::new(AbortOnDropSubscription { abort }) as Box<dyn Subscription>
                });

            OutputPayload::stream(subscribe)
        }
    }

    let graph = crate::build_flow!(
        nodes: [("stream", CancelStreamNode)],
        edges: []
    );

    let (mut runner, _handle) = crate::flow::Runner::new(graph, None);
    runner.set_start_node("stream", OutputPayload::cloned(()));

    let (tx, mut rx) = tokio::sync::mpsc::channel(20);
    runner.set_event_sender(tx);

    let runner_task = tokio::spawn(async move { runner.run().await });

    let mut completed_received = false;
    let mut next_received = false;

    while let Some(event) = rx.recv().await {
        match event {
            FlowEvent::NodeCompleted(id) if id == "stream" => {
                completed_received = true;
            }
            FlowEvent::NodeStreamNextMessage(id, _) if id == "stream" => {
                next_received = true;
            }
            FlowEvent::FlowFinished => break,
            _ => {}
        }
    }

    runner_task.await.unwrap().unwrap();

    assert!(next_received, "Should receive stream message");
    assert!(completed_received, "Should receive NodeCompleted event");
}

#[tokio::test]
async fn test_parallel_stream_nodes_all_completed_before_flow_finished() {
    use crate::flow::{FlowEvent, ReactiveStream};
    use std::collections::HashSet;

    struct StreamNode;
    #[async_trait]
    impl crate::Node for StreamNode {
        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            let stream = ReactiveStream::from_observable(vec!["A".to_string()]);
            OutputPayload::stream(stream.subscribe)
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("s1", StreamNode),
            ("s2", StreamNode),
            ("s3", StreamNode),
            ("s4", StreamNode),
        ],
        edges: []
    );

    let (mut runner, _handle) = crate::flow::Runner::new(graph, None);
    runner.set_start_node("s1", OutputPayload::cloned(()));
    runner.set_start_node("s2", OutputPayload::cloned(()));
    runner.set_start_node("s3", OutputPayload::cloned(()));
    runner.set_start_node("s4", OutputPayload::cloned(()));

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    runner.set_event_sender(tx);

    let runner_task = tokio::spawn(async move { runner.run().await });

    let mut started = HashSet::new();
    let mut completed = HashSet::new();

    while let Some(event) = rx.recv().await {
        match event {
            FlowEvent::NodeStarted(id) => {
                started.insert(id);
            }
            FlowEvent::NodeCompleted(id) => {
                completed.insert(id);
            }
            FlowEvent::FlowFinished => break,
            _ => {}
        }
    }

    runner_task.await.unwrap().unwrap();

    assert_eq!(started, completed, "All started nodes should complete before finish");
}
