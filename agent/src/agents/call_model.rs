use async_stream::stream;
use futures::{Stream, StreamExt};

use crate::agents::{ToolCall, ToolDef, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

/// 工具执行结果
#[derive(Debug, Clone)]
pub struct CallToolResult {
    /// 工具调用 ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行是否成功
    pub success: bool,
    /// 输出内容（JSON 字符串）
    pub output: String,
}

/// 调用模型事件（纯数据）
#[derive(Debug, Clone)]
pub enum CallModelEvent {
    /// LLM 文本片段
    TextChunk(String),
    /// 推理片段
    ReasoningChunk(String),
    /// LLM 响应完成
    Completed {
        /// 完整的响应内容
        content: String,
        /// 完整的推理内容（如果有）
        reasoning_content: Option<String>,
        /// 工具调用列表（如果有）
        tools_call: Option<Vec<ToolCall>>,
    },
    Error(String),
}

// 调用一个具备 chat 能力的模型，至少要实现 chat_stream 方法
pub fn call_model(
    model: &(dyn ChatCapability + Sync),
    messages: &[Message],
    tools_def: Option<&[ToolDef]>,
) -> impl Stream<Item = CallModelEvent> {
    let mut final_content = String::new();
    let mut final_reasoning_content = String::new();
    let mut final_tool_calls = Vec::new();
    stream! {
        // 流式调用模型
        let mut response_stream = match model
            .chat_stream(messages.to_vec(), tools_def.map(|tools| tools.to_vec()))
            .await
        {
            Ok(s) => s,
            Err(e) => {
                yield CallModelEvent::Error(e.to_string());
                return;
            }
        };
        while let Some(chunk) = response_stream.next().await {
                if !chunk.content.is_empty() {
                    final_content.push_str(&chunk.content);
                    yield CallModelEvent::TextChunk(chunk.content);
                }

                if !chunk.reasoning_content.is_empty() {
                    final_reasoning_content.push_str(&chunk.reasoning_content);
                    yield CallModelEvent::ReasoningChunk(chunk.reasoning_content);
                }

                if let Some(inc_tool_calls) = chunk.tool_calls {
                    merge_tool_calls(&mut final_tool_calls, inc_tool_calls);
                }
        }

        yield CallModelEvent::Completed {
            content: final_content,
            reasoning_content: if final_reasoning_content.is_empty() { None } else { Some(final_reasoning_content) },
            tools_call: if final_tool_calls.is_empty() { None } else { Some(final_tool_calls) },
        };
    }
}

pub async fn call_tool(tool_executor: &dyn ToolExecutor, call: &ToolCall) -> CallToolResult {
    let call_id = call.id.clone();
    let tool_name = call.get_name();

    let result = tool_executor.execute(call).await;

    let (success, output) = match result {
        Ok(r) if r.success => {
            let output = serde_json::to_string(&r.output).unwrap_or_else(|_| "{}".to_string());
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

    CallToolResult {
        call_id,
        tool_name,
        success,
        output,
    }
}

// 并行执行多个工具
pub fn call_tools(
    tool_executor: &dyn ToolExecutor,
    tools_call: &[ToolCall],
) -> impl Stream<Item = CallToolResult> {
    let futures: Vec<_> = tools_call
        .iter()
        .map(|call| call_tool(tool_executor, call))
        .collect();
    stream! {
        let results = futures::future::join_all(futures).await;
        for result in results {
            yield result;
        }
    }
}

/// 合并增量 ToolCall
/// 流式响应中 tool_calls 是增量发送的，需要按 index 合并
fn merge_tool_calls(accumulated: &mut Vec<ToolCall>, incremental: Vec<ToolCall>) {
    for inc in incremental {
        // 查找是否已存在相同 index 或 id 的 tool call
        let existing = accumulated.iter_mut().find(|tc| {
            // 优先按 index 匹配，其次按 id 匹配
            if let (Some(idx1), Some(idx2)) = (tc.index, inc.index) {
                idx1 == idx2
            } else {
                !tc.id.is_empty() && tc.id == inc.id
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
