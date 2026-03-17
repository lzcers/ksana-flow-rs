use std::time::Duration;

use super::AgentActor;
use crate::agents::hooks::{AgentHook, HookRegistry};
use crate::agents::{Context, TimeoutPolicyHook, ToolExecutor};
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
    hooks: HookRegistry,
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
            hooks: HookRegistry::default(),
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

    /// 替换 hooks 注册表
    pub fn hooks(mut self, hooks: HookRegistry) -> Self {
        self.hooks = hooks;
        self
    }

    /// 追加一个 hook
    pub fn hook<H>(mut self, hook: H) -> Self
    where
        H: AgentHook + 'static,
    {
        self.hooks.push(hook);
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
        let mut actor =
            AgentActor::with_hooks(self.chat, self.tool_executor, self.context, self.hooks);
        actor.state.max_iterations = self.max_iterations;
        actor.state.user_id = self.user_id;

        if self.step_timeout.is_some() || self.tool_timeout.is_some() {
            let mut timeout_hook = TimeoutPolicyHook::new();
            if let Some(timeout) = self.step_timeout {
                timeout_hook = timeout_hook.step_timeout(timeout);
            }
            if let Some(timeout) = self.tool_timeout {
                timeout_hook = timeout_hook.tool_timeout(timeout);
            }
            actor.add_hook(timeout_hook);
        }

        actor
    }
}
