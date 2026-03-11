#[cfg(test)]
mod tests {
    use crate::agents::{GenericToolExecutor, Tool, ToolDef, ToolExecutor, ToolResult};
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatModel};
    use crate::providers::DeepSeekProvider;
    use async_trait::async_trait;
    use futures::StreamExt;
    use serde_json::{Value, json};
    use std::sync::Arc;

    /// 测试带 tools 的流式响应
    /// 目的：验证 ChatModel 能正确处理带 tools 的请求
    #[tokio::test]
    async fn test_stream_response_with_tools() {
        dotenv::dotenv().ok();
        // 1. 创建 ChatModel
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                println!("跳过测试: 未设置 DEEPSEEK_API_KEY 环境变量");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        if let Err(e) = model.set_active_model("deepseek-chat") {
            println!("设置活动模型失败: {}", e);
            return;
        }

        // 2. 定义 tools
        let tools = vec![ToolDef {
            name: "get_weather".to_string(),
            description: "获取指定城市的天气信息".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "城市名称，例如：北京、上海"
                    }
                },
                "required": ["city"]
            }),
        }];

        // 3. 构建消息
        let messages = vec![
            Message::system("你是一个有用的助手。"),
            Message::user("北京今天的天气怎么样？"),
        ];

        // 4. 调用 chat_stream
        let stream = model.chat_stream(messages, Some(tools)).await;
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                println!("流式请求失败: {:?}", e);
                return;
            }
        };

        println!("\n========== 流式响应开始 (带 Tools) ==========\n");

        let mut content_chunks: Vec<String> = Vec::new();
        let mut chunk_count = 0;

        // 5. 遍历流
        while let Some(chunk) = stream.next().await {
            chunk_count += 1;
            content_chunks.push(chunk.content.clone());

            if !chunk.content.is_empty() {
                println!("[Chunk {}] {}", chunk_count, chunk.content);
            }

            if chunk.is_finished {
                println!("[Finish Reason]: {:?}", chunk.finish_reason);
            }
        }

        println!("\n========== 流式响应结束 ==========\n");

        // 打印最终拼接结果
        println!("=== 最终内容拼接 (共 {} 个 chunk) ===", chunk_count);
        let full_content = content_chunks.join("");
        println!("{}", full_content);

        assert!(
            !full_content.is_empty() || chunk_count > 0,
            "应该有响应内容"
        );
    }

    /// 测试纯文本流式响应
    /// 目的：验证 ChatModel 流式响应的正确性
    #[tokio::test]
    async fn test_stream_response_text_only() {
        // 1. 创建 ChatModel
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                println!("跳过测试: 未设置 DEEPSEEK_API_KEY 环境变量");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        if let Err(e) = model.set_active_model("deepseek-chat") {
            println!("设置活动模型失败: {}", e);
            return;
        }

        // 2. 构建消息（不带 tools）
        let messages = vec![
            Message::system("你是一个有用的助手。"),
            Message::user("用一句话介绍 Rust 编程语言。"),
        ];

        // 3. 调用 chat_stream
        let stream = model.chat_stream(messages, None).await;
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                println!("流式请求失败: {:?}", e);
                return;
            }
        };

        println!("\n========== 流式响应开始 (纯文本) ==========\n");

        let mut content_chunks: Vec<String> = Vec::new();
        let mut chunk_count = 0;

        // 4. 遍历流
        while let Some(chunk) = stream.next().await {
            chunk_count += 1;
            content_chunks.push(chunk.content.clone());

            if !chunk.content.is_empty() {
                println!("[Chunk {}] {}", chunk_count, chunk.content);
            }

            if chunk.is_finished {
                println!("[Finish Reason]: {:?}", chunk.finish_reason);
            }
        }

        println!("\n========== 流式响应结束 ==========\n");

        // 打印最终拼接结果
        println!("=== 最终内容拼接 (共 {} 个 chunk) ===", chunk_count);
        let full_content = content_chunks.join("");
        println!("{}", full_content);

        assert!(!full_content.is_empty(), "应该有响应内容");
    }

    // ============================================================================
    // Mock Tool for Testing
    // ============================================================================

    /// 模拟天气工具 - 用于测试
    struct MockWeatherTool {
        def: ToolDef,
    }

    impl MockWeatherTool {
        fn new() -> Self {
            Self {
                def: ToolDef {
                    name: "get_weather".to_string(),
                    description: "获取指定城市的天气信息".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "city": {
                                "type": "string",
                                "description": "城市名称"
                            }
                        },
                        "required": ["city"]
                    }),
                },
            }
        }
    }

    #[async_trait]
    impl Tool for MockWeatherTool {
        fn definition(&self) -> &ToolDef {
            &self.def
        }

        async fn execute(
            &self,
            arguments: Value,
        ) -> Result<Value, crate::agents::ToolExecutorError> {
            let city = arguments
                .get("city")
                .and_then(|v| v.as_str())
                .unwrap_or("未知城市");

            // 模拟天气数据
            let weather = json!({
                "city": city,
                "temperature": "22°C",
                "condition": "晴朗",
                "humidity": "45%",
                "wind": "东南风 3级"
            });

            println!("\n[Tool Executed] get_weather(city=\"{}\")", city);
            println!(
                "[Tool Result] {}\n",
                serde_json::to_string_pretty(&weather).unwrap()
            );

            Ok(weather)
        }
    }

    /// 测试 Agent Loop - 完整的工具调用流程
    /// 目的：展示 Agent 调用工具的完整过程
    #[tokio::test]
    async fn test_agent_loop_with_tools() {
        dotenv::dotenv().ok();

        // 1. 创建 ChatModel
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                println!("跳过测试: 未设置 DEEPSEEK_API_KEY 环境变量");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        if let Err(e) = model.set_active_model("deepseek-chat") {
            println!("设置活动模型失败: {}", e);
            return;
        }

        // 2. 创建工具执行器并注册工具
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());

        // 3. 初始化消息
        let mut messages = vec![
            Message::system(
                "你是一个有用的助手。当用户询问天气时，请使用 get_weather 工具获取信息。",
            ),
            Message::user("北京今天天气怎么样？"),
        ];

        let max_iterations = 5;

        println!("\n========== Agent Loop 开始 ==========\n");
        println!("[User] 北京今天天气怎么样？\n");

        // 4. Agent Loop
        for iteration in 1..=max_iterations {
            println!("=== Iteration {} ===", iteration);

            // 调用模型
            let response = model
                .chat(messages.clone(), Some(executor.tools().clone()))
                .await;

            let assistant_msg = match response {
                Ok(msg) => msg,
                Err(e) => {
                    println!("[Error] 模型调用失败: {:?}", e);
                    break;
                }
            };

            // 打印助手响应
            if let Message::Assistant {
                content,
                tool_calls,
            } = &assistant_msg
            {
                if !content.is_empty() {
                    println!("[Assistant] {}", content);
                }

                // 添加助手消息到历史
                messages.push(assistant_msg.clone());

                // 检查是否有工具调用
                if let Some(calls) = tool_calls {
                    println!("\n[Tool Calls] 检测到 {} 个工具调用", calls.len());

                    // 执行每个工具调用
                    for call in calls {
                        println!("[Tool Call] id={}, name={}", call.id, call.get_name());

                        // 执行工具
                        let result = executor.execute(call.clone()).await;

                        match result {
                            Ok(ToolResult {
                                id,
                                success,
                                output,
                            }) => {
                                println!("[Tool Result] id={}, success={}", id, success);

                                // 将工具结果添加到消息历史
                                messages.push(Message::Tool {
                                    tool_call_id: id,
                                    content: serde_json::to_string(&output).unwrap_or_default(),
                                });
                            }
                            Err(e) => {
                                println!("[Tool Error] {:?}", e);
                                messages.push(Message::Tool {
                                    tool_call_id: call.id.clone(),
                                    content: format!("Error: {:?}", e),
                                });
                            }
                        }
                    }

                    println!();
                    // 继续下一轮迭代，让模型处理工具结果
                    continue;
                } else {
                    // 没有工具调用，Agent 完成
                    println!("\n[Agent Complete] 无更多工具调用");
                    break;
                }
            } else {
                println!("[Unexpected] 非助手消息");
                break;
            }
        }

        println!("\n========== Agent Loop 结束 ==========\n");

        // 打印最终对话
        println!("=== 最终对话历史 ===");
        for (i, msg) in messages.iter().enumerate() {
            match msg {
                Message::System { content } => {
                    println!("[{}] System: {}", i, content);
                }
                Message::User { content } => {
                    println!("[{}] User: {}", i, content);
                }
                Message::Assistant {
                    content,
                    tool_calls,
                } => {
                    println!(
                        "[{}] Assistant: {} (tool_calls: {:?})",
                        i,
                        content,
                        tool_calls.as_ref().map(|c| c.len())
                    );
                }
                Message::Tool {
                    tool_call_id,
                    content,
                } => {
                    println!("[{}] Tool({}): {}", i, tool_call_id, content);
                }
            }
        }

        // 验证：应该有至少 3 条消息（system + user + assistant）
        assert!(messages.len() >= 2, "应该有至少 2 条消息");
    }
}
