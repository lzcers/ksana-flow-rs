use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::agent::{
    CallToolResult,
    agent_actor::lifecycle::{LifeCycleInterrupt, StepFrame},
    tools::ToolCall,
};

/// Agent 执行错误类型
#[derive(Debug, Clone, thiserror::Error, Deserialize, Serialize)]
pub enum AgentError {
    /// 用户取消
    #[error("Cancelled by user")]
    Cancelled,
    /// 操作超时
    #[error("Operation timed out")]
    Timeout,
    #[error("Model response is not expected")]
    ModelRspErr,

    #[error("life cycle error: {0}")]
    Parse(#[from] LifeCycleInterrupt),
}

/// Actor 产生的事件
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AgentActorEvent {
    /// 流式输出片段
    ContentChunk(String),
    /// 推理内容片段（DeepSeek 推理模式）
    ReasoningChunk(String),
    /// LLM 响应完成，包含完整的 content 和 tool_calls；此时 step 尚未提交
    StepCompleted {
        /// 完整的响应内容
        content: String,
        /// 完整的推理内容（如果有）
        reasoning_content: Option<String>,
        /// 工具调用列表（如果有）
        tool_calls: Option<Vec<ToolCall>>,
    },
    /// 单步结果已提交，表示最终落地到状态/上下文的结果
    StepFinalized {
        result: StepResult,
        frame: StepFrame,
    },
    /// 模型请求工具调用
    ToolCalls(Vec<ToolCall>),
    /// 单个工具执行完成
    ToolResult {
        call_id: String,
        success: bool,
        output: String,
    },
    /// 一次迭代完成
    Iteration {
        iteration: usize,
        message_count: usize,
    },
    /// 公共 hook 发送的自定义事件
    HookEvent {
        hook: String,
        kind: String,
        payload: Value,
    },
    /// 当前 step 提交后触发的 max-iterations 终态事件
    MaxIterations { iteration: usize },
    /// `run_loop()` 在 `Done` step 完成提交后发送的 actor 级终态事件
    Completed,
    /// 用户取消执行；可能来自 step 提交，也可能来自 loop 在步间取消
    Cancelled,
    /// 当前 step 提交后触发的错误终态事件
    Error(AgentError),
    /// 请求用户输入
    AskUser {
        /// 问题内容
        question: String,
        /// 输入 ID，用于匹配响应
        input_id: String,
    },
}

/// 发送给 Actor 的控制命令
#[derive(Debug)]
pub enum AgentActorCommand {
    /// 暂停执行
    Pause,
    /// 继续执行（恢复）
    Continue,
    /// 取消执行
    Cancel,
    /// 提供用户输入
    UserInput {
        /// 输入内容
        input: String,
        /// 输入 ID，用于匹配请求
        input_id: String,
    },
}

/// Actor 的外部控制句柄
pub struct AgentActorHandle {
    /// 命令发送器
    pub cmd_tx: mpsc::Sender<AgentActorCommand>,
    /// 事件接收器
    pub event_rx: mpsc::Receiver<AgentActorEvent>,
}

impl AgentActorHandle {
    /// 暂停 Actor
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Pause).await;
    }

    /// 继续/恢复 Actor
    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Continue).await;
    }

    /// 取消 Actor
    pub async fn cancel(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Cancel).await;
    }

    /// 提供用户输入
    pub async fn provide_input(&self, input: String, input_id: String) {
        let _ = self
            .cmd_tx
            .send(AgentActorCommand::UserInput { input, input_id })
            .await;
    }

    /// 检查 actor 是否已完成（事件 channel 已关闭）
    pub fn is_finished(&self) -> bool {
        self.event_rx.is_closed()
    }

    /// 等待 Actor 完成，返回所有事件
    pub async fn wait(mut self) -> Vec<AgentActorEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.event_rx.recv().await {
            let is_terminal = matches!(
                event,
                AgentActorEvent::Completed
                    | AgentActorEvent::Cancelled
                    | AgentActorEvent::Error(_)
                    | AgentActorEvent::MaxIterations { .. }
            );
            events.push(event);
            if is_terminal {
                break;
            }
        }
        events
    }
}

/// 单步执行结果
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum StepResult {
    /// 需要继续执行（有工具调用）
    Continue {
        content: String,
        reasoning_content: Option<String>,
        tools_call: Vec<ToolCall>,
        tools_result: Vec<CallToolResult>,
    },
    /// 执行完成（无工具调用）
    Done {
        content: String,
        reasoning_content: Option<String>,
    },
    /// 执行出错
    Error(AgentError),
}
