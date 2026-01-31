use super::*;
use crate::flow::Graph;

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
