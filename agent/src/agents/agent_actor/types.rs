use serde_json::Value;
use tokio::sync::mpsc;

use crate::agents::hooks::{HookPhase, StepResultDraft};
use crate::agents::tools::ToolCall;

/// Agent 执行错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    /// 模型调用错误
    #[error("Model error: {0}")]
    Model(String),
    /// 工具执行错误
    #[error("Tool execution error: {0}")]
    Tool(String),
    /// Hook 执行错误
    #[error("Hook `{plugin}` failed at {phase:?}: {message}")]
    Hook {
        plugin: &'static str,
        phase: HookPhase,
        message: String,
    },
    /// 用户取消
    #[error("Cancelled by user")]
    Cancelled,
    /// 操作超时
    #[error("Operation timed out")]
    Timeout,
    /// 达到最大迭代次数
    #[error("Max iterations ({0}) reached")]
    MaxIterations(usize),
}

/// Actor 产生的事件
#[derive(Debug, Clone)]
pub enum AgentActorEvent {
    /// 流式输出片段
    ContentChunk(String),
    /// 推理内容片段（DeepSeek 推理模式）
    ReasoningChunk(String),
    /// LLM 响应完成，包含完整的 content 和 tool_calls
    StepCompleted {
        /// 完整的响应内容
        content: String,
        /// 完整的推理内容（如果有）
        reasoning_content: Option<String>,
        /// 工具调用列表（如果有）
        tool_calls: Option<Vec<ToolCall>>,
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
    /// 达到最大迭代次数
    MaxIterations { iteration: usize },
    /// Agent 执行完成
    Completed,
    /// 用户取消执行
    Cancelled,
    /// 发生错误
    Error(AgentError),
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
#[derive(Debug, Clone)]
pub enum StepResult {
    /// 需要继续执行（有工具调用）
    Continue {
        content: String,
        reasoning_content: Option<String>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<crate::agents::call_model::CallToolResult>,
    },
    /// 执行完成（无工具调用）
    Done {
        content: String,
        reasoning_content: Option<String>,
    },
    /// 执行出错
    Error(AgentError),
}

pub(super) fn step_result_from_draft(draft: StepResultDraft) -> StepResult {
    match draft {
        StepResultDraft::Continue {
            content,
            reasoning_content,
            tool_calls,
            tool_results,
        } => StepResult::Continue {
            content,
            reasoning_content,
            tool_calls,
            tool_results,
        },
        StepResultDraft::Done {
            content,
            reasoning_content,
        } => StepResult::Done {
            content,
            reasoning_content,
        },
        StepResultDraft::Error(err) => StepResult::Error(err),
    }
}
