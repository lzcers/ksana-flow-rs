#[cfg(test)]
mod tests {
    use crate::agents::utils::collect_stream;
    use crate::agents::{GenericToolExecutor, Tool, ToolDef, ToolExecutor, call_tools};
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatModel};
    use crate::providers::DeepSeekProvider;
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;

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

    /// 测试 Agent Loop - 流式工具调用流程
    /// 目的：展示 Agent 流式调用工具的完整过程
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

        println!("\n========== Agent Loop (Stream Mode) 开始 ==========\n");
        println!("[User] 北京今天天气怎么样？\n");

        // 4. Agent Loop
        for iteration in 1..=max_iterations {
            println!("=== Iteration {} ===", iteration);

            // 流式调用模型
            let stream = model
                .chat_stream(messages.clone(), Some(executor.tools().clone()))
                .await;

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    println!("[Error] 模型调用失败: {:?}", e);
                    break;
                }
            };

            // 收集流式响应
            let (content, tool_calls) = collect_stream(stream).await;

            // 构建助手消息
            let assistant_msg = Message::Assistant {
                content,
                tool_calls: tool_calls.clone(),
            };

            // 打印助手响应（流式已实时打印，这里打印摘要）
            println!("\n[Assistant Message] 已接收完整响应");

            // 添加助手消息到历史
            messages.push(assistant_msg);

            // 检查是否有工具调用
            if let Some(calls) = tool_calls {
                let tools_result = call_tools(&executor, calls).await;
                messages.extend(tools_result);
                continue;
            } else {
                // 没有工具调用，Agent 完成
                println!("\n[Agent Complete] 无更多工具调用");
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

        // 验证：应该有至少 2 条消息（system + user）
        assert!(messages.len() >= 2, "应该有至少 2 条消息");
    }
}
