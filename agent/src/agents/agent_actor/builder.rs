use std::time::Duration;

use super::AgentActor;
use crate::agents::{Context, ToolExecutor};
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
    step_timeout: Option<Duration>,
    tool_timeout: Option<Duration>,
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
            step_timeout: None,
            tool_timeout: None,
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

    /// 设置步骤超时时间
    pub fn step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = Some(timeout);
        self
    }

    /// 设置工具执行超时时间
    pub fn tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = Some(timeout);
        self
    }

    /// 构建 AgentActor
    pub fn build(self) -> AgentActor<C, E> {
        let mut actor = AgentActor::with_runtime_hooks(self.chat, self.tool_executor, self.context);
        actor.state.max_iterations = self.max_iterations;
        actor.state.user_id = self.user_id;
        actor.step_timeout = self.step_timeout;
        actor.tool_timeout = self.tool_timeout;

        actor
    }
}
