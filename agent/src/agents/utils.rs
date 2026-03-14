use futures::StreamExt;

use crate::{
    agents::{ToolCall, ToolExecutor},
    core::Message,
    models::ChatChunk,
};

/// 合并增量 ToolCall
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
                    // 合并 name
                    if !inc_func.name.is_empty() {
                        existing_func.name = inc_func.name.clone();
                    }
                    // 追加 arguments（增量字符串）
                    existing_func.arguments.push_str(&inc_func.arguments);
                } else {
                    existing.function = Some(inc_func.clone());
                }
            }
            // 合并简化格式字段
            if let Some(inc_name) = &inc.name {
                existing.name = Some(inc_name.clone());
            }
            if let Some(inc_args) = &inc.arguments {
                if let Some(existing_args) = &mut existing.arguments {
                    // 如果现有是 Object，尝试合并
                    if existing_args.is_object() && inc_args.is_object() {
                        if let (Some(existing_obj), Some(inc_obj)) =
                            (existing_args.as_object_mut(), inc_args.as_object())
                        {
                            for (k, v) in inc_obj {
                                existing_obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                } else {
                    existing.arguments = Some(inc_args.clone());
                }
            }
        } else {
            // 新增 tool call
            accumulated.push(inc);
        }
    }
}

/// 收集流式响应，返回累积的 content 和 tool_calls
pub async fn collect_stream(
    stream: futures::stream::BoxStream<'static, ChatChunk>,
) -> (String, Option<Vec<ToolCall>>) {
    let mut content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut finish_reason: Option<String> = None;

    println!("[Stream] 开始接收流式响应...");

    let mut stream = stream;
    while let Some(chunk) = stream.next().await {
        // 累积内容
        if !chunk.content.is_empty() {
            print!("{}", chunk.content); // 实时打印
            content.push_str(&chunk.content);
        }

        // 合并增量 tool_calls
        if let Some(inc_tool_calls) = chunk.tool_calls {
            merge_tool_calls(&mut tool_calls, inc_tool_calls);
        }

        // 记录结束原因
        if let Some(reason) = &chunk.finish_reason {
            finish_reason = Some(reason.clone());
        }
    }

    println!(); // 换行

    let final_tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    println!(
        "[Stream] 结束，finish_reason: {:?}, tool_calls: {:?}",
        finish_reason, final_tool_calls
    );

    (content, final_tool_calls)
}

/// 执行工具调用并返回 Tool 消息列表
///
/// # Arguments
/// * `executor` - 工具执行器
/// * `tool_calls` - 要执行的工具调用列表
///
/// # Returns
/// 返回 `Vec<Message::Tool>`，每个消息对应一个工具调用的结果。
/// 如果工具执行成功，content 为 JSON 输出字符串；
/// 如果执行失败，content 包含错误信息。
pub async fn call_tools(executor: &dyn ToolExecutor, tool_calls: Vec<ToolCall>) -> Vec<Message> {
    let mut messages = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
        let tool_call_id = call.id.clone();
        let result = executor.execute(call).await;

        let content = match result {
            Ok(tool_result) => {
                if tool_result.success {
                    serde_json::to_string(&tool_result.output).unwrap_or_else(|_| {
                        r#"{"error": "Failed to serialize tool output"}"#.to_string()
                    })
                } else {
                    serde_json::to_string(&serde_json::json!({
                        "error": tool_result.output
                    }))
                    .unwrap_or_else(|_| r#"{"error": "Tool execution failed"}"#.to_string())
                }
            }
            Err(e) => serde_json::to_string(&serde_json::json!({
                "error": e.to_string()
            }))
            .unwrap_or_else(|_| r#"{"error": "Tool execution error"}"#.to_string()),
        };

        messages.push(Message::Tool {
            tool_call_id,
            content,
        });
    }

    messages
}
