use super::AgentActor;
use crate::agent::{Context, ToolExecutor};
use crate::models::ChatCapability;

/// Agent Actor 构建器
pub struct AgentActorBuilder<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    chat: C,
    tool_executor: E,
    context: Context,
    max_iterations: usize,
    user_id: String,
}

impl<C, E> AgentActorBuilder<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// 创建新的构建器
    pub fn new(chat: C, tool_executor: E) -> Self {
        Self {
            chat,
            tool_executor,
            context: Context::new(),
            max_iterations: 10,
            user_id: "default".to_string(),
        }
    }

    /// 设置上下文
    pub fn context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    /// 设置最大迭代次数
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// 设置用户 ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = user_id.into();
        self
    }

    /// 构建 AgentActor
    pub fn build(self) -> AgentActor<C, E> {
        let mut actor = AgentActor::with_runtime_hooks(self.chat, self.tool_executor, self.context);
        actor.state.metrics.execution.max_iterations = self.max_iterations;
        actor.state.user_id = self.user_id;
        actor
    }
}
