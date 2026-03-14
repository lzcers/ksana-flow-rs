//! Agent 执行器 - 纯粹的单步执行逻辑
//!
//! 无状态的纯执行单元：
//! - 输入：messages + event_tx
//! - 输出：StepResult
//! - 不依赖 Context，不维护任何状态
//!
//! Context 的更新由调用方（AgentActor）负责。

use std::sync::Arc;

use futures::{StreamExt, stream::BoxStream};
use tokio::sync::mpsc;

use crate::agents::{ToolCall, ToolExecutor};
use crate::core::Message;
use crate::models::{ChatCapability, ChatChunk};

// ============================================================================
// 执行结果类型
// ============================================================================

/// 流式输出片段
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// LLM 输出的文本片段
    Text(String),
    /// DeepSeek 推理模式的推理内容片段
    Reasoning(String),
    /// LLM 响应完成
    Completed {
        /// 完整的响应内容
        content: String,
        /// 完整的推理内容（如果有）
        reasoning_content: Option<String>,
        /// 工具调用列表（如果有）
        tool_calls: Option<Vec<ToolCall>>,
    },
}

/// 单步执行结果
#[derive(Debug, Clone)]
pub enum StepResult {
    /// 需要继续迭代（有工具调用）
    Continue {
        /// LLM 响应内容
        content: String,
        /// 推理内容（DeepSeek 推理模式）
        reasoning_content: Option<String>,
        /// 工具调用列表
        tool_calls: Vec<ToolCall>,
        /// 工具执行结果
        tool_results: Vec<ToolExecutionResult>,
    },
    /// 执行完成（无工具调用）
    Done {
        /// LLM 响应内容
        content: String,
        /// 推理内容（DeepSeek 推理模式）
        reasoning_content: Option<String>,
    },
    /// 执行错误
    Error(String),
}

/// 工具执行结果（简化版，不含事件发送）
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// 工具调用 ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行是否成功
    pub success: bool,
    /// 输出内容（JSON 字符串）
    pub output: String,
}

// ============================================================================
// Agent 执行器
// ============================================================================

/// Agent 执行器 - 无状态的纯执行单元
///
/// 执行器只依赖 ChatCapability 和 ToolExecutor，不维护任何状态。
/// 它只执行单步迭代，返回结构化结果。
///
/// # 设计原则
/// - 纯函数式：输入 messages + event_tx，输出 StepResult
/// - 无状态：不持有 Context，不管理对话历史
/// - 状态管理由调用方（如 AgentActor）负责
pub struct AgentExecutor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send + Sync,
{
    /// Chat 模型
    chat: Arc<C>,
    /// 工具执行器
    executor: Arc<E>,
}

impl<C, E> AgentExecutor<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + Sync + 'static,
{
    /// 创建执行器
    pub fn new(chat: Arc<C>, executor: Arc<E>) -> Self {
        Self { chat, executor }
    }

    /// 执行单步迭代，返回结构化结果
    ///
    /// 这是一个纯函数：输入消息列表，输出执行结果。
    /// 不修改任何内部状态，不管理对话历史。
    ///
    /// # 参数
    /// - `messages`: 完整的消息列表（包含系统消息、对话历史等）
    /// - `stream_tx`: 可选的流式事件发送通道，用于接收 `StreamChunk` 事件
    ///
    /// # 返回
    /// - `StepResult::Continue`: 有工具调用，可能需要继续迭代
    /// - `StepResult::Done`: 无工具调用，执行完成
    /// - `StepResult::Error`: 执行出错
    ///
    /// # 注意
    /// 调用方需要：
    /// 1. 在调用前准备好消息列表（从 Context 获取）
    /// 2. 在调用后根据结果更新 Context（添加 assistant 消息、tool 消息）
    pub async fn run_step(
        &self,
        messages: Vec<Message>,
        stream_tx: Option<mpsc::Sender<StreamChunk>>,
    ) -> StepResult {
        // 获取工具定义
        let tools = self.executor.tools().clone();

        // 流式调用模型
        let stream = match self.chat.chat_stream(messages, Some(tools)).await {
            Ok(s) => s,
            Err(e) => return StepResult::Error(e.to_string()),
        };

        // 收集流式响应并实时发送事件
        let (content, reasoning_content, tool_calls) =
            collect_stream(stream, stream_tx.as_ref()).await;

        // 发送 LLM 响应完成事件
        if let Some(ref tx) = stream_tx {
            if let Err(e) = tx
                .send(StreamChunk::Completed {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: tool_calls.clone(),
                })
                .await
            {
                return StepResult::Error(format!("Failed to send LlmResponse event: {}", e));
            }
        }

        // 检查是否有工具调用
        if let Some(tool_calls) = tool_calls {
            // 执行工具
            let tool_results = self.execute_tools(&tool_calls).await;

            StepResult::Continue {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            }
        } else {
            // 没有工具调用，完成
            StepResult::Done {
                content,
                reasoning_content,
            }
        }
    }

    /// 执行工具调用并返回执行结果列表
    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Vec<ToolExecutionResult> {
        let mut results = Vec::with_capacity(tool_calls.len());

        for call in tool_calls {
            let call_id = call.id.clone();
            let tool_name = call.get_name();

            let result = self.executor.execute(call.clone()).await;

            let (success, output) = match result {
                Ok(r) if r.success => {
                    let output =
                        serde_json::to_string(&r.output).unwrap_or_else(|_| "{}".to_string());
                    (true, output)
                }
                Ok(r) => {
                    let error_msg = serde_json::to_string(&serde_json::json!({
                        "error": r.output
                    }))
                    .unwrap_or_else(|_| r#"{\"error\": \"Tool execution failed\"}"#.to_string());
                    (false, error_msg)
                }
                Err(e) => {
                    let error_msg = serde_json::to_string(&serde_json::json!({
                        "error": e.to_string()
                    }))
                    .unwrap_or_else(|_| r#"{\"error\": \"Tool execution error\"}"#.to_string());
                    (false, error_msg)
                }
            };

            results.push(ToolExecutionResult {
                call_id,
                tool_name,
                success,
                output,
            });
        }

        results
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 收集流式响应，实时发送文本片段，返回累积的内容和工具调用
///
/// # 参数
/// - `stream`: LLM 返回的流式响应
/// - `stream_tx`: 可选的流式事件发送通道
///
/// # 返回
/// - `(content, reasoning_content, tool_calls)`: 累积的内容、推理内容和工具调用列表
async fn collect_stream(
    mut stream: BoxStream<'static, ChatChunk>,
    stream_tx: Option<&mpsc::Sender<StreamChunk>>,
) -> (String, Option<String>, Option<Vec<ToolCall>>) {
    let mut content = String::new();
    let mut reasoning_content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    while let Some(chunk) = stream.next().await {
        // 实时发送文本片段
        if !chunk.content.is_empty() {
            if let Some(tx) = stream_tx {
                let _ = tx.send(StreamChunk::Text(chunk.content.clone())).await;
            }
            content.push_str(&chunk.content);
        }

        // 实时发送推理内容片段
        if !chunk.reasoning_content.is_empty() {
            if let Some(tx) = stream_tx {
                let _ = tx
                    .send(StreamChunk::Reasoning(chunk.reasoning_content.clone()))
                    .await;
            }
            reasoning_content.push_str(&chunk.reasoning_content);
        }

        // 合并工具调用
        if let Some(inc_tool_calls) = chunk.tool_calls {
            merge_tool_calls(&mut tool_calls, inc_tool_calls);
        }
    }

    let final_reasoning = if reasoning_content.is_empty() {
        None
    } else {
        Some(reasoning_content)
    };

    let final_tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    (content, final_reasoning, final_tool_calls)
}

/// 合并增量 ToolCall
///
/// 流式响应中 tool_calls 是增量发送的，需要按 index 合并
fn merge_tool_calls(accumulated: &mut Vec<ToolCall>, incremental: Vec<ToolCall>) {
    for inc in incremental {
        // 查找是否已存在相同 index 或 id 的 tool call
        let existing = accumulated.iter_mut().find(|tc| {
            // 优先按 index 匹配，其次按 id 匹配
            if let (Some(idx1), Some(idx2)) = (tc.index, inc.index) {
                idx1 == idx2
            } else if !tc.id.is_empty() && tc.id == inc.id {
                true
            } else {
                false
            }
        });

        if let Some(existing) = existing {
            // 合并增量数据
            if !inc.id.is_empty() {
                existing.id = inc.id;
            }
            if inc.call_type.is_some() {
                existing.call_type = inc.call_type;
            }
            if inc.index.is_some() {
                existing.index = inc.index;
            }
            // 合并 function 字段
            if let Some(inc_func) = &inc.function {
                if let Some(existing_func) = &mut existing.function {
                    if !inc_func.name.is_empty() {
                        existing_func.name = inc_func.name.clone();
                    }
                    existing_func.arguments.push_str(&inc_func.arguments);
                } else {
                    existing.function = Some(inc_func.clone());
                }
            }
        } else {
            // 新增 tool call
            accumulated.push(inc);
        }
    }
}
