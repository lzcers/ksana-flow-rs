use super::{
    graph::{Graph, NodeId},
    keys::{CTX_INPUT, CTX_SUBGRAPH_ID, CTX_SUBGRAPH_INPUT, INPUT_EXTERNAL_START},
};
use crate::{
    Context, ControllerHandle, ControllerRunners, RunnerId, RunnerKind,
    flow::runner::ExecutionContext, scope_runner, try_controller, try_current_node_id,
    try_runner_id,
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::timeout;

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
pub struct SubgraphExecutor {
    /// 子图定义, 一个子图执行器可被多个图共享，因此使用 Arc 引用计数
    subgraph: Arc<Graph>,
    /// 执行配置
    config: SubgraphConfig,
}

impl Clone for SubgraphExecutor {
    fn clone(&self) -> Self {
        Self {
            subgraph: self.subgraph.clone(),
            config: self.config.clone(),
        }
    }
}

impl SubgraphExecutor {
    /// 创建新的子图执行器
    pub fn new(subgraph: Graph, config: SubgraphConfig) -> Self {
        Self {
            subgraph: Arc::new(subgraph),
            config,
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
        let subgraph_ctx = self.create_context(input.clone(), parent_ctx);
        let parent_node_id = try_current_node_id();

        let (runner_id, mut runner, _handle) = controller.create_runner(
            self.subgraph.clone(),
            Some(ExecutionContext::new()),
            RunnerKind::Subgraph,
            parent_runner_id,
            parent_node_id,
        );

        runner.set_runtime_context(subgraph_ctx);

        let mut inputs = HashMap::new();
        inputs.insert(INPUT_EXTERNAL_START.to_string(), input);
        runner.set_start_node(&self.config.entry_node, inputs.into());

        let execution_task = scope_runner(controller.clone(), runner_id, runner.run());

        let result = if let Some(duration) = self.config.timeout {
            match timeout(duration, execution_task).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(SubgraphError::ExecutionFailed {
                    message: format!("Runner error: {}", e),
                }),
                Err(_) => Err(SubgraphError::Timeout { limit: duration }),
            }
        } else {
            match execution_task.await {
                Ok(()) => Ok(()),
                Err(e) => Err(SubgraphError::ExecutionFailed {
                    message: format!("Runner error: {}", e),
                }),
            }
        };

        let output = if result.is_ok() {
            runner
                .get_execution_context()
                .get_output(&self.config.exit_node)
                .unwrap_or(Value::Null)
        } else {
            return Err(result.err().unwrap());
        };

        controller.unregister_runner(runner_id);
        Ok(output)
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
        }
    }
}

impl std::error::Error for SubgraphError {}

#[cfg(test)]
mod tests {
    use super::*;

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
}
