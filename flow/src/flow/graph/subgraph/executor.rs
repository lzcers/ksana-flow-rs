use super::{
    graph::{Graph, NodeId},
    keys::{CTX_INPUT, CTX_SUBGRAPH_ID, CTX_SUBGRAPH_INPUT, INPUT_EXTERNAL_START},
};
use crate::{
    Context, ControllerHandle, ControllerRunners, RunnerId, RunnerKind,
    flow::runner::{ExecutionContext, NodeState},
    scope_runner, try_controller, try_current_node_id, try_runner_id,
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::timeout;

/// Owns a Controller registration for the duration of one inline subgraph run.
///
/// Unlike runners started through `ControllerRunners::spawn_runner`, subgraph runners are
/// executed inline, so the caller must unregister them. Keeping that responsibility in `Drop`
/// covers success, validation failures after creation, runner errors, and timeout cancellation.
struct RunnerRegistrationGuard {
    controller: ControllerHandle,
    runner_id: RunnerId,
}

impl RunnerRegistrationGuard {
    fn new(controller: ControllerHandle, runner_id: RunnerId) -> Self {
        Self {
            controller,
            runner_id,
        }
    }
}

impl Drop for RunnerRegistrationGuard {
    fn drop(&mut self) {
        self.controller.unregister_runner(self.runner_id);
    }
}

/// 子图执行器配置
#[derive(Debug, Clone)]
pub struct SubgraphConfig {
    /// 执行超时
    pub timeout: Option<Duration>,
    /// 是否继承父上下文
    pub inherit_context: bool,
    /// 入口节点 ID（子图的起始节点）
    pub entry_node: NodeId,
    /// 出口节点 ID（子图的结束节点）
    pub exit_node: NodeId,
}

impl Default for SubgraphConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            inherit_context: false,
            entry_node: "start".to_string(),
            exit_node: "end".to_string(),
        }
    }
}

/// 子图执行器
/// 复用现有 Runner 实现子图执行，支持上下文隔离/继承
#[derive(Clone)]
pub struct SubgraphExecutor {
    /// 子图定义, 一个子图执行器可被多个图共享，因此使用 Arc 引用计数
    subgraph: Arc<Graph>,
    /// 执行配置
    config: SubgraphConfig,
    /// Graph 和配置均不可变时，缓存校验结果，避免高 fan-out 重复扫描节点表。
    validation_error: Option<SubgraphError>,
}

impl SubgraphExecutor {
    /// 创建新的子图执行器
    pub fn new(subgraph: Graph, config: SubgraphConfig) -> Self {
        let validation_error = Self::validate_config_for(&subgraph, &config).err();
        Self {
            subgraph: Arc::new(subgraph),
            config,
            validation_error,
        }
    }

    /// 使用默认配置创建子图执行器
    pub fn with_defaults(subgraph: Graph) -> Self {
        Self::new(subgraph, SubgraphConfig::default())
    }

    /// 执行子图
    /// # 参数
    /// - `input`: 子图输入数据
    /// - `parent_ctx`: 父图上下文
    /// # 返回
    /// 子图执行结果
    pub async fn execute(
        &self,
        input: Value,
        parent_ctx: &Context,
    ) -> Result<Value, SubgraphError> {
        let controller = try_controller().ok_or_else(|| SubgraphError::ExecutionFailed {
            message: "Missing Controller in current task scope".to_string(),
        })?;
        self.execute_with_controller(input, parent_ctx, controller)
            .await
    }

    pub async fn execute_with_controller(
        &self,
        input: Value,
        parent_ctx: &Context,
        controller: ControllerHandle,
    ) -> Result<Value, SubgraphError> {
        self.execute_with_controller_and_parent(input, parent_ctx, controller, try_runner_id())
            .await
    }

    pub async fn execute_with_controller_and_parent(
        &self,
        input: Value,
        parent_ctx: &Context,
        controller: ControllerHandle,
        parent_runner_id: Option<RunnerId>,
    ) -> Result<Value, SubgraphError> {
        self.validate_config()?;

        let subgraph_ctx = self.create_context(input.clone(), parent_ctx);
        let parent_node_id = try_current_node_id();

        let (runner_id, mut runner, _handle) = controller.create_runner(
            self.subgraph.clone(),
            Some(ExecutionContext::new()),
            RunnerKind::Subgraph,
            parent_runner_id,
            parent_node_id,
        );
        let _registration = RunnerRegistrationGuard::new(controller.clone(), runner_id);

        runner.set_runtime_context(subgraph_ctx);

        let mut inputs = HashMap::new();
        inputs.insert(INPUT_EXTERNAL_START.to_string(), input);
        runner.set_start_node(&self.config.entry_node, inputs.into());

        let execution_task = scope_runner(controller.clone(), runner_id, runner.run());

        if let Some(duration) = self.config.timeout {
            match timeout(duration, execution_task).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    return Err(SubgraphError::ExecutionFailed {
                        message: format!("Runner error: {}", e),
                    });
                }
                Err(_) => return Err(SubgraphError::Timeout { limit: duration }),
            }
        } else {
            if let Err(e) = execution_task.await {
                return Err(SubgraphError::ExecutionFailed {
                    message: format!("Runner error: {}", e),
                });
            }
        }

        let execution_context = runner.get_execution_context();
        let exit_state = execution_context.get_state(&self.config.exit_node);
        if exit_state != Some(NodeState::Completed) {
            return Err(SubgraphError::ExitNotCompleted {
                node_id: self.config.exit_node.clone(),
                state: exit_state,
            });
        }

        execution_context
            .get_output(&self.config.exit_node)
            .ok_or_else(|| SubgraphError::MissingExitOutput {
                node_id: self.config.exit_node.clone(),
            })
    }

    fn validate_config(&self) -> Result<(), SubgraphError> {
        match &self.validation_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn validate_config_for(subgraph: &Graph, config: &SubgraphConfig) -> Result<(), SubgraphError> {
        for (field, node_id) in [
            ("entry_node", &config.entry_node),
            ("exit_node", &config.exit_node),
        ] {
            if !subgraph.nodes.contains_key(node_id) {
                return Err(SubgraphError::InvalidConfig {
                    field: field.to_string(),
                    reason: format!("node '{}' does not exist in the subgraph", node_id),
                });
            }
        }
        Ok(())
    }

    /// 创建子图执行上下文
    fn create_context(&self, input: Value, parent_ctx: &Context) -> Context {
        if self.config.inherit_context {
            // 继承父上下文并添加子图隔离
            let ctx = parent_ctx.child();
            // 添加子图隔离标记
            ctx.set(CTX_SUBGRAPH_ID, self.config.entry_node.clone());
            ctx.set(CTX_SUBGRAPH_INPUT, input);
            ctx
        } else {
            // 完全隔离的新上下文
            let ctx = Context::new();
            // 将输入放入上下文
            ctx.set(CTX_INPUT, input);
            ctx
        }
    }

    /// 获取配置
    pub fn config(&self) -> &SubgraphConfig {
        &self.config
    }

    /// 更新配置
    pub fn set_config(&mut self, config: SubgraphConfig) {
        self.validation_error = Self::validate_config_for(self.subgraph.as_ref(), &config).err();
        self.config = config;
    }
}

/// 子图执行错误
#[derive(Debug, Clone)]
pub enum SubgraphError {
    /// 执行失败
    ExecutionFailed { message: String },
    /// 执行超时
    Timeout { limit: Duration },
    /// 无效配置
    InvalidConfig { field: String, reason: String },
    /// Runner 已结束，但出口节点未成功完成
    ExitNotCompleted {
        node_id: NodeId,
        state: Option<NodeState>,
    },
    /// 出口节点已完成，但没有生成输出
    MissingExitOutput { node_id: NodeId },
}

impl std::fmt::Display for SubgraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubgraphError::ExecutionFailed { message } => {
                write!(f, "Subgraph execution failed: {}", message)
            }
            SubgraphError::Timeout { limit } => {
                write!(f, "Subgraph execution timeout after {:?}", limit)
            }
            SubgraphError::InvalidConfig { field, reason } => {
                write!(f, "Invalid subgraph config: {} - {}", field, reason)
            }
            SubgraphError::ExitNotCompleted { node_id, state } => {
                write!(
                    f,
                    "Subgraph exit node '{}' did not complete (state: {:?})",
                    node_id, state
                )
            }
            SubgraphError::MissingExitOutput { node_id } => {
                write!(f, "Subgraph exit node '{}' produced no output", node_id)
            }
        }
    }
}

impl std::error::Error for SubgraphError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Controller, Input, Node, Output};
    use async_trait::async_trait;
    use serde_json::json;

    struct EchoNode;

    #[async_trait]
    impl Node for EchoNode {
        async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
            Ok(input.get_any().cloned().unwrap_or(Value::Null).into())
        }
    }

    struct EmptyNode;

    #[async_trait]
    impl Node for EmptyNode {
        async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
            Ok(Output::new(None))
        }
    }

    struct ErrorNode;

    #[async_trait]
    impl Node for ErrorNode {
        async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
            Err("expected failure".to_string())
        }
    }

    struct SlowNode;

    #[async_trait]
    impl Node for SlowNode {
        async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(Value::Null.into())
        }
    }

    #[test]
    fn test_subgraph_executor_creation() {
        let graph = Graph::new();
        let executor = SubgraphExecutor::with_defaults(graph);

        assert_eq!(executor.config().timeout, None);
        assert!(!executor.config().inherit_context);
    }

    #[test]
    fn test_subgraph_config_defaults() {
        let config = SubgraphConfig::default();

        assert_eq!(config.timeout, None);
        assert!(!config.inherit_context);
        assert_eq!(config.entry_node, "start");
        assert_eq!(config.exit_node, "end");
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

    #[test]
    fn set_config_refreshes_cached_validation() {
        let mut graph = Graph::new();
        graph.add_node("entry", || EchoNode);
        graph.add_node("exit", || EchoNode);
        let mut executor = SubgraphExecutor::with_defaults(graph);
        assert!(executor.validate_config().is_err());

        executor.set_config(SubgraphConfig {
            entry_node: "entry".to_string(),
            exit_node: "exit".to_string(),
            ..SubgraphConfig::default()
        });

        assert!(executor.validate_config().is_ok());
    }

    #[tokio::test]
    async fn rejects_missing_entry_and_exit_nodes_before_registering_runner() {
        let mut graph = Graph::new();
        graph.add_node("present", || EchoNode);
        let ctx = Context::new();

        let executor = SubgraphExecutor::new(
            graph.clone(),
            SubgraphConfig {
                entry_node: "missing-entry".to_string(),
                exit_node: "present".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let (controller, rx) = Controller::new();
        drop(rx);
        let error = executor
            .execute_with_controller(json!(1), &ctx, controller.clone())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SubgraphError::InvalidConfig { ref field, .. } if field == "entry_node"
        ));
        assert!(controller.get_runner_handle(1).is_none());

        let executor = SubgraphExecutor::new(
            graph,
            SubgraphConfig {
                entry_node: "present".to_string(),
                exit_node: "missing-exit".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let error = executor
            .execute_with_controller(json!(1), &ctx, controller.clone())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SubgraphError::InvalidConfig { ref field, .. } if field == "exit_node"
        ));
        assert!(controller.get_runner_handle(1).is_none());
    }

    #[tokio::test]
    async fn rejects_exit_that_did_not_complete_and_unregisters_runner() {
        let mut graph = Graph::new();
        graph.add_node("start", || EchoNode);
        graph.add_node("end", || EchoNode);
        let executor = SubgraphExecutor::with_defaults(graph);
        let (controller, rx) = Controller::new();
        drop(rx);

        let error = executor
            .execute_with_controller(json!(1), &Context::new(), controller.clone())
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            SubgraphError::ExitNotCompleted {
                ref node_id,
                state: None
            } if node_id == "end"
        ));
        assert!(controller.get_runner_handle(1).is_none());
    }

    #[tokio::test]
    async fn rejects_completed_exit_without_output_and_unregisters_runner() {
        let mut graph = Graph::new();
        graph.add_node("end", || EmptyNode);
        let executor = SubgraphExecutor::new(
            graph,
            SubgraphConfig {
                entry_node: "end".to_string(),
                exit_node: "end".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let (controller, rx) = Controller::new();
        drop(rx);

        let error = executor
            .execute_with_controller(Value::Null, &Context::new(), controller.clone())
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            SubgraphError::MissingExitOutput { ref node_id } if node_id == "end"
        ));
        assert!(controller.get_runner_handle(1).is_none());
    }

    #[tokio::test]
    async fn unregisters_runner_after_success() {
        let mut graph = Graph::new();
        graph.add_node("end", || EchoNode);
        let executor = SubgraphExecutor::new(
            graph,
            SubgraphConfig {
                entry_node: "end".to_string(),
                exit_node: "end".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let (controller, rx) = Controller::new();
        drop(rx);

        let output = executor
            .execute_with_controller(json!({"ok": true}), &Context::new(), controller.clone())
            .await
            .unwrap();

        assert_eq!(output, json!({"ok": true}));
        assert!(controller.get_runner_handle(1).is_none());
    }

    #[tokio::test]
    async fn unregisters_runner_after_execution_failure() {
        let mut graph = Graph::new();
        graph.add_node("start", || ErrorNode);
        let executor = SubgraphExecutor::new(
            graph,
            SubgraphConfig {
                exit_node: "start".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let (controller, rx) = Controller::new();
        drop(rx);

        let error = executor
            .execute_with_controller(Value::Null, &Context::new(), controller.clone())
            .await
            .unwrap_err();

        assert!(matches!(error, SubgraphError::ExecutionFailed { .. }));
        assert!(controller.get_runner_handle(1).is_none());
    }

    #[tokio::test]
    async fn unregisters_runner_after_timeout() {
        let mut graph = Graph::new();
        graph.add_node("start", || SlowNode);
        let executor = SubgraphExecutor::new(
            graph,
            SubgraphConfig {
                timeout: Some(Duration::from_millis(1)),
                exit_node: "start".to_string(),
                ..SubgraphConfig::default()
            },
        );
        let (controller, rx) = Controller::new();
        drop(rx);

        let error = executor
            .execute_with_controller(Value::Null, &Context::new(), controller.clone())
            .await
            .unwrap_err();

        assert!(matches!(error, SubgraphError::Timeout { .. }));
        assert!(controller.get_runner_handle(1).is_none());
    }
}
