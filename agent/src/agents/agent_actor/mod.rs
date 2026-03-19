//! `agent_actor` 模块负责 Agent 的高层编排，对外暴露统一的执行入口、
//! 控制句柄与构建 API。
//!
//! 实现按职责拆分在以下子模块中：
//! - `types`: 定义 Actor 对外可见的数据结构，包括错误类型、运行事件、
//!   控制命令、控制句柄，以及单步执行结果。
//! - `lifecycle`: 实现单步执行的生命周期内部流程，负责 hook 调度、
//!   模型调用流处理、工具调用以及 step 结果组装。
//! - `commit`: 实现单步最终结果的 reducer / committer，把状态迁移、
//!   上下文持久化和提交事件统一收口。
//! - `loop_control`: 实现 `run_step`、后台循环执行与暂停、继续、取消等
//!   控制逻辑，对应 Actor 的对外运行控制行为。
//! - `builder`: 实现 `AgentActorBuilder`，负责 Actor 初始化参数装配，
//!   包括上下文、迭代次数、用户 ID 与超时策略 hook。
//!
//! 当前文件作为目录模块入口，只保留共享状态定义、公共导出以及少量
//! 跨子模块复用的辅助方法。

/// 构建器相关实现位于 `builder` 子模块。
mod builder;
/// 单步提交 reducer / committer 位于 `commit` 子模块。
mod commit;
/// 单步执行生命周期位于 `lifecycle` 子模块。
mod lifecycle;
/// 后台循环控制逻辑位于 `loop_control` 子模块。
mod loop_control;
/// 错误、事件、命令和结果类型位于 `types` 子模块。
mod types;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::hooks::{Hook, HookRegistry, RuntimeHook, RuntimeHookRegistry};
use crate::agents::{AgentState, Context, ToolExecutor};
use crate::models::ChatCapability;

pub use builder::AgentActorBuilder;
pub use types::{AgentActorCommand, AgentActorEvent, AgentActorHandle, AgentError, StepResult};

/// Agent Actor 只负责组合模型、工具执行器和 hooks。
///
/// 具体的 step 执行、loop 控制与 builder 逻辑分别位于子模块中，
/// 这里保留核心状态和最小化的公共 API。
pub struct AgentActor<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// Agent 状态（控制面）
    state: AgentState,
    /// Chat 模型
    chat: Arc<C>,
    /// 工具执行器
    tool_executor: Arc<E>,
    /// 内部生命周期 hooks
    runtime_hooks: RuntimeHookRegistry,
    /// 面向外部扩展的 hooks
    hooks: HookRegistry,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
    /// 创建新的 Agent Actor
    pub fn new(chat: C, tool_executor: E, context: Context) -> Self {
        Self::with_runtime_hooks(
            chat,
            tool_executor,
            context,
            RuntimeHookRegistry::default(),
            HookRegistry::default(),
        )
    }

    /// 创建带自定义扩展 hooks 的 Agent Actor
    pub fn with_hooks(chat: C, tool_executor: E, context: Context, hooks: HookRegistry) -> Self {
        Self::with_runtime_hooks(
            chat,
            tool_executor,
            context,
            RuntimeHookRegistry::default(),
            hooks,
        )
    }

    /// 创建带内部 runtime hooks 和扩展 hooks 的 Agent Actor
    pub(crate) fn with_runtime_hooks(
        chat: C,
        tool_executor: E,
        context: Context,
        runtime_hooks: RuntimeHookRegistry,
        hooks: HookRegistry,
    ) -> Self {
        Self {
            state: AgentState {
                job_id: Uuid::new_v4(),
                user_id: "default".to_string(),
                conversation_id: None,
                title: String::new(),
                description: String::new(),
                category: None,
                state: crate::agents::JobState::Pending,
                budget: None,
                actual_cost: rust_decimal::Decimal::ZERO,
                context,
                iteration: 0,
                max_iterations: 10,
            },
            chat: Arc::new(chat),
            tool_executor: Arc::new(tool_executor),
            runtime_hooks,
            hooks,
        }
    }

    /// 获取状态
    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// 获取可变状态
    pub fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }

    /// 获取上下文
    pub fn context(&self) -> &Context {
        &self.state.context
    }

    /// 获取可变上下文
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.state.context
    }

    /// 获取扩展 hooks 注册表
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// 获取可变扩展 hooks 注册表
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// 读取单个 hook 的状态快照
    pub fn hook_snapshot(&self, name: &str) -> Option<Value> {
        self.runtime_hooks.snapshot(name)
    }

    /// 读取所有支持快照的 hook 状态
    pub fn hook_snapshots(&self) -> HashMap<String, Value> {
        self.runtime_hooks.snapshots()
    }

    /// 追加一个扩展 hook
    pub fn add_hook<H>(&mut self, hook: H)
    where
        H: Hook + 'static,
    {
        self.hooks.push(hook);
    }

    /// 追加一个内部 runtime hook
    pub(crate) fn add_runtime_hook<H>(&mut self, hook: H)
    where
        H: RuntimeHook + 'static,
    {
        self.runtime_hooks.push(hook);
    }

    /// 发送事件（忽略发送失败）
    async fn send_event(event_tx: Option<&mpsc::Sender<AgentActorEvent>>, event: AgentActorEvent) {
        if let Some(tx) = event_tx {
            let _ = tx.send(event).await;
        }
    }
}
