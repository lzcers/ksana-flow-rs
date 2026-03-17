//! Actor 控制器 - 高层编排与 API 暴露
//!
//! 具体职责拆分：
//! - `types`: 错误、事件、命令、句柄、StepResult
//! - `runtime`: 单步执行与 hook 驱动运行时
//! - `loop_control`: 后台循环与暂停/继续/取消控制
//! - `builder`: 构建器与超时策略装配

mod builder;
mod loop_control;
mod runtime;
mod types;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::hooks::{AgentHook, HookRegistry};
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
    /// 生命周期 hooks
    hooks: HookRegistry,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
    /// 创建新的 Agent Actor
    pub fn new(chat: C, tool_executor: E, context: Context) -> Self {
        Self::with_hooks(chat, tool_executor, context, HookRegistry::default())
    }

    /// 创建带自定义 hooks 的 Agent Actor
    pub fn with_hooks(chat: C, tool_executor: E, context: Context, hooks: HookRegistry) -> Self {
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

    /// 获取 hooks 注册表
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// 获取可变 hooks 注册表
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// 读取单个 hook 的状态快照
    pub fn hook_snapshot(&self, name: &str) -> Option<Value> {
        self.hooks.snapshot(name)
    }

    /// 读取所有支持快照的 hook 状态
    pub fn hook_snapshots(&self) -> HashMap<String, Value> {
        self.hooks.snapshots()
    }

    /// 追加一个 hook
    pub fn add_hook<H>(&mut self, hook: H)
    where
        H: AgentHook + 'static,
    {
        self.hooks.push(hook);
    }

    /// 发送事件（忽略发送失败）
    async fn send_event(event_tx: Option<&mpsc::Sender<AgentActorEvent>>, event: AgentActorEvent) {
        if let Some(tx) = event_tx {
            let _ = tx.send(event).await;
        }
    }
}
