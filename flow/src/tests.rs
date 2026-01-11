use crate::flow::{Context, Node};
use crate::flow::{GraphBuilder, Runner};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_complex_graph_connections() {
    struct InputNode;
    #[async_trait]

    impl Node for InputNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
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
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("{}[{}]", input, self.branch_id)
        }
    }

    struct MergeNode;
    #[async_trait]

    impl Node for MergeNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("Merged: {}", input)
        }
    }

    struct OutputNode;
    #[async_trait]

    impl Node for OutputNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    let graph = GraphBuilder::new()
        .add_node("input", InputNode)
        .add_node("branch1", BranchNode::new("A"))
        .add_node("branch2", BranchNode::new("B"))
        .add_node("branch3", BranchNode::new("C"))
        .add_node("merge1", MergeNode)
        .add_node("merge2", MergeNode)
        .add_node("output", OutputNode)
        .add_edge("input", "branch1")
        .add_edge("input", "branch2")
        .add_edge("input", "branch3")
        .add_edge("branch1", "merge1")
        .add_edge("branch2", "merge1")
        .add_edge("merge1", "merge2")
        .add_edge("branch3", "merge2")
        .add_edge("merge2", "output")
        .build();

    let mut runner = Runner::new(graph).set_start_node("input", &"Test".to_string());
    let result = runner.run().await;

    assert!(result.is_ok(), "Complex graph execution should succeed");
}

#[tokio::test]
async fn test_build_flow_macro() {
    struct TestNode;
    #[async_trait]
    impl Node for TestNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    let graph = crate::build_flow!(
        nodes: [
            ("node2", TestNode),
            ("node3", TestNode),
        ],
        edges: [
            ("node1", "node2"),
            ("node2", "node3"),
        ]
    );

    let mut runner = Runner::new(graph).set_start_node("node1", &"Start".to_string());
    let result = runner.run().await;
    assert!(result.is_ok());

    let is_true = |_ctx: &Context, _out: &String| true;
    let graph_cond = crate::build_flow!(
        nodes: [
            ("node2", TestNode),
        ],
        edges: [
            ("node1", "node2", is_true),
        ]
    );
    let mut runner_cond = Runner::new(graph_cond).set_start_node("node1", &"Start".to_string());
    let result_cond = runner_cond.run().await;
    assert!(result_cond.is_ok());
}

#[tokio::test]
async fn test_conditional_branching() {
    struct InputNode;
    #[async_trait]

    impl Node for InputNode {
        type In = i32;
        type Out = i32;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    struct CheckNode;
    #[async_trait]

    impl Node for CheckNode {
        type In = i32;
        type Out = (i32, bool);
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            (input, input > 0)
        }
    }

    struct PositiveNode;
    #[async_trait]

    impl Node for PositiveNode {
        type In = (i32, bool);
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("Positive: {}", input.0)
        }
    }

    struct NegativeNode;
    #[async_trait]

    impl Node for NegativeNode {
        type In = (i32, bool);
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("Negative: {}", input.0)
        }
    }

    let is_positive = |_ctx: &Context, output: &(i32, bool)| output.1;
    let is_negative = |_ctx: &Context, output: &(i32, bool)| !output.1;

    let graph = GraphBuilder::new()
        .add_node("input", InputNode)
        .add_node("check", CheckNode)
        .add_node("positive", PositiveNode)
        .add_node("negative", NegativeNode)
        .add_edge("input", "check")
        .add_condition_edge("check", "positive", is_positive)
        .add_condition_edge("check", "negative", is_negative)
        .build();

    let mut runner = Runner::new(graph).set_start_node("input", &42);
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
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("L{}: {}", self.level, input)
        }
    }

    let graph = GraphBuilder::new()
        .add_node("l1_1", LevelNode::new(1))
        .add_node("l1_2", LevelNode::new(1))
        .add_node("l2_1", LevelNode::new(2))
        .add_node("l2_2", LevelNode::new(2))
        .add_node("l2_3", LevelNode::new(2))
        .add_node("l3_1", LevelNode::new(3))
        .add_node("l3_2", LevelNode::new(3))
        .add_node("l4_1", LevelNode::new(4))
        .add_edge("l1_1", "l2_1")
        .add_edge("l1_1", "l2_2")
        .add_edge("l1_2", "l2_2")
        .add_edge("l1_2", "l2_3")
        .add_edge("l2_1", "l3_1")
        .add_edge("l2_2", "l3_1")
        .add_edge("l2_2", "l3_2")
        .add_edge("l2_3", "l3_2")
        .add_edge("l3_1", "l4_1")
        .add_edge("l3_2", "l4_1")
        .build();

    let mut runner = Runner::new(graph).set_start_node("l1_1", &"Start".to_string());
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
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("{}[Node{}]", input, self.id)
        }
    }

    struct AggregatorNode;
    #[async_trait]

    impl Node for AggregatorNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("Aggregated: {}", input)
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
    let mut runner = Runner::new(graph).set_start_node("input", &"Data".to_string());
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
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            let mut log = self.execution_log.lock().unwrap();
            log.push(self.id);
            std::thread::sleep(Duration::from_millis(10));
            format!("{}[{}]", input, self.id)
        }
    }

    struct CollectorNode;
    #[async_trait]

    impl Node for CollectorNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
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
    let mut runner = Runner::new(graph).set_start_node("input", &"Concurrent".to_string());
    let result = runner.run().await;

    assert!(result.is_ok(), "Concurrent execution should succeed");

    let log = execution_log.lock().unwrap();
    assert_eq!(log.len(), 21, "All nodes should have executed");
}

#[tokio::test]
async fn test_Context_write_and_read() {
    struct WriterNode;
    #[async_trait]

    impl Node for WriterNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("key1", "value1");
            ctx.set("key2", 42);
            ctx.set("key3", vec![1, 2, 3]);
            input
        }
    }

    struct ReaderNode;
    #[async_trait]

    impl Node for ReaderNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out {
            let value1: Option<String> = ctx.get("key1");
            let value2: Option<i32> = ctx.get("key2");
            let value3: Option<Vec<i32>> = ctx.get("key3");

            assert_eq!(value1, Some("value1".to_string()));
            assert_eq!(value2, Some(42));
            assert_eq!(value3, Some(vec![1, 2, 3]));

            format!("Read: {}", input)
        }
    }

    let graph = GraphBuilder::new()
        .add_node("writer", WriterNode)
        .add_node("reader", ReaderNode)
        .add_edge("writer", "reader")
        .build();

    let mut runner = Runner::new(graph).set_start_node("writer", &"Test".to_string());
    let result = runner.run().await;

    assert!(result.is_ok(), "&Context read/write should succeed");
}

#[tokio::test]
async fn test_Context_across_multiple_nodes() {
    struct Node1;
    #[async_trait]

    impl Node for Node1 {
        type In = String;
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("node1_data", "from_node1");
            ctx.set("counter", 1);
            input
        }
    }

    struct Node2;
    #[async_trait]

    impl Node for Node2 {
        type In = String;
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            let counter: Option<i32> = ctx.get("counter");
            if let Some(c) = counter {
                ctx.set("counter", c + 1);
            }
            ctx.set("node2_data", "from_node2");
            input
        }
    }

    struct Node3;
    #[async_trait]

    impl Node for Node3 {
        type In = String;
        type Out = String;
        async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out {
            let node1_data: Option<String> = ctx.get("node1_data");
            let node2_data: Option<String> = ctx.get("node2_data");
            let counter: Option<i32> = ctx.get("counter");

            assert_eq!(node1_data, Some("from_node1".to_string()));
            assert_eq!(node2_data, Some("from_node2".to_string()));
            assert_eq!(counter, Some(2));

            format!("Final: {}", input)
        }
    }

    let graph = GraphBuilder::new()
        .add_node("node1", Node1)
        .add_node("node2", Node2)
        .add_node("node3", Node3)
        .add_edge("node1", "node2")
        .add_edge("node2", "node3")
        .build();

    let mut runner = Runner::new(graph).set_start_node("node1", &"Start".to_string());
    let result = runner.run().await;

    assert!(result.is_ok(), "&Context across multiple nodes should work");
}

#[tokio::test]
async fn test_Context_with_conditional_edges() {
    struct InputNode;
    #[async_trait]

    impl Node for InputNode {
        type In = i32;
        type Out = i32;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("input_value", input);
            input
        }
    }

    struct CheckNode;
    #[async_trait]

    impl Node for CheckNode {
        type In = i32;
        type Out = (i32, bool);
        async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out {
            let stored: Option<i32> = ctx.get("input_value");
            assert_eq!(stored, Some(input));
            (input, input % 2 == 0)
        }
    }

    struct EvenNode;
    #[async_trait]

    impl Node for EvenNode {
        type In = (i32, bool);
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("result", "even");
            format!("Even: {}", input.0)
        }
    }

    struct OddNode;
    #[async_trait]

    impl Node for OddNode {
        type In = (i32, bool);
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("result", "odd");
            format!("Odd: {}", input.0)
        }
    }

    let is_even = |_ctx: &Context, output: &(i32, bool)| output.1;
    let is_odd = |_ctx: &Context, output: &(i32, bool)| !output.1;

    let graph = GraphBuilder::new()
        .add_node("input", InputNode)
        .add_node("check", CheckNode)
        .add_node("even", EvenNode)
        .add_node("odd", OddNode)
        .add_edge("input", "check")
        .add_condition_edge("check", "even", is_even)
        .add_condition_edge("check", "odd", is_odd)
        .build();

    let mut runner = Runner::new(graph).set_start_node("input", &4);
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "&Context with conditional edges should work"
    );
}

#[tokio::test]
async fn test_Context_serialization() {
    struct SerializeNode;
    #[async_trait]

    impl Node for SerializeNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, mut ctx: &Context, input: Self::In) -> Self::Out {
            ctx.set("string", "hello");
            ctx.set("number", 123);
            ctx.set("float", 45.67);
            ctx.set("boolean", true);
            ctx.set("array", vec![1, 2, 3, 4, 5]);
            input
        }
    }

    struct DeserializeNode;
    #[async_trait]

    impl Node for DeserializeNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out {
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

            format!("Deserialized: {}", input)
        }
    }

    let graph = GraphBuilder::new()
        .add_node("serialize", SerializeNode)
        .add_node("deserialize", DeserializeNode)
        .add_edge("serialize", "deserialize")
        .build();

    let mut runner = Runner::new(graph).set_start_node("serialize", &"Test".to_string());
    let result = runner.run().await;

    assert!(result.is_ok(), "&Context serialization should work");
}

#[tokio::test]
async fn test_diamond_graph_pattern() {
    struct StartNode;
    #[async_trait]

    impl Node for StartNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    struct LeftNode;
    #[async_trait]

    impl Node for LeftNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("L: {}", input)
        }
    }

    struct RightNode;
    #[async_trait]

    impl Node for RightNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("R: {}", input)
        }
    }

    struct EndNode;
    #[async_trait]

    impl Node for EndNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            format!("E: {}", input)
        }
    }

    let graph = GraphBuilder::new()
        .add_node("start", StartNode)
        .add_node("left", LeftNode)
        .add_node("right", RightNode)
        .add_node("end", EndNode)
        .add_edge("start", "left")
        .add_edge("start", "right")
        .add_edge("left", "end")
        .add_edge("right", "end")
        .build();

    let mut runner = Runner::new(graph).set_start_node("start", &"Diamond".to_string());
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "Diamond pattern should execute successfully"
    );
}

#[tokio::test]
async fn test_type_mismatch() {
    struct StringNode;
    #[async_trait]

    impl Node for StringNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    struct IntNode;
    #[async_trait]

    impl Node for IntNode {
        type In = i32;
        type Out = i32;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    let graph = GraphBuilder::new()
        .add_node("string_node", StringNode)
        .add_node("int_node", IntNode)
        .add_edge("string_node", "int_node")
        .build();

    let mut runner = Runner::new(graph).set_start_node("string_node", &"Test".to_string());
    let result = runner.run().await;
    eprintln!("{:?}", result);
    assert!(result.is_err(), "Type mismatch should cause error");
}

#[tokio::test]
async fn test_unit_input_allows_any() {
    struct StringNode;
    #[async_trait]
    impl Node for StringNode {
        type In = String;
        type Out = String;
        async fn run(&mut self, _ctx: &Context, input: Self::In) -> Self::Out {
            input
        }
    }

    struct UnitNode;
    #[async_trait]
    impl Node for UnitNode {
        type In = ();
        type Out = String;
        async fn run(&mut self, _ctx: &Context, _input: Self::In) -> Self::Out {
            "UnitNode called".to_string()
        }
    }

    let graph = GraphBuilder::new()
        .add_node("string_node", StringNode)
        .add_node("unit_node", UnitNode)
        .add_edge("string_node", "unit_node")
        .build();

    let mut runner = Runner::new(graph).set_start_node("string_node", &"Test".to_string());
    let result = runner.run().await;

    assert!(
        result.is_ok(),
        "Unit input node should allow any input type, got error: {:?}",
        result.err()
    );
}
