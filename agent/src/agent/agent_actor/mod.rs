/// 构建器相关实现位于 `builder` 子模块。
mod builder;
/// 单步执行生命周期位于 `lifecycle` 子模块。
pub mod lifecycle;
/// 后台循环控制逻辑位于 `loop_control` 子模块。
mod loop_control;
/// 错误、事件、命令和结果类型位于 `types` 子模块。
mod types;

use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent::{AgentState, Context, Metrics, ToolExecutor};
use crate::models::ChatCapability;

pub use builder::AgentActorBuilder;
pub use loop_control::LoopState;
pub use types::{AgentActorCommand, AgentActorEvent, AgentActorHandle, AgentError, StepResult};

/// Agent Actor 负责组合模型、工具执行器和运行配置。
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
    /// 等待中的用户输入
    pending_user_input: Option<(String, mpsc::Sender<AgentActorEvent>)>,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
    /// 创建新的 Agent Actor
    pub fn new(chat: C, tool_executor: E, context: Context) -> Self {
        Self::with_runtime_hooks(chat, tool_executor, context)
    }

    /// 创建带内部 runtime hooks 和扩展 hooks 的 Agent Actor
    pub(crate) fn with_runtime_hooks(chat: C, tool_executor: E, context: Context) -> Self {
        let default_max_iterations = 10;
        Self {
            state: AgentState {
                job_id: Uuid::new_v4(),
                user_id: "default".to_string(),
                conversation_id: None,
                title: String::new(),
                description: String::new(),
                category: None,
                state: crate::agent::JobState::Pending,
                context,
                metrics: Metrics::with_max_iterations(default_max_iterations),
            },
            chat: Arc::new(chat),
            tool_executor: Arc::new(tool_executor),
            pending_user_input: None,
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

    /// 发送事件（忽略发送失败）
    async fn send_event(event_tx: Option<&mpsc::Sender<AgentActorEvent>>, event: AgentActorEvent) {
        if let Some(tx) = event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// 触发用户输入请求
    pub async fn ask_user(
        &mut self,
        question: String,
        event_tx: Option<mpsc::Sender<AgentActorEvent>>,
    ) -> LoopState {
        let input_id = Uuid::new_v4().to_string();

        if let Some(tx) = &event_tx {
            Self::send_event(
                Some(tx),
                AgentActorEvent::AskUser {
                    question,
                    input_id: input_id.clone(),
                },
            )
            .await;
        }

        self.state.state = crate::agent::JobState::Paused;
        self.pending_user_input = Some((input_id.clone(), event_tx.unwrap()));
        LoopState::WaitingForUserInput(input_id)
    }

    /// 检查是否有等待中的用户输入
    pub fn has_pending_user_input(&self) -> bool {
        self.pending_user_input.is_some()
    }
}
