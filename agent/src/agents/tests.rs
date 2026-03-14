#[cfg(test)]
mod tests {
    use crate::agents::tools::playwright_cli::PlaywrightCliTool;
    use crate::agents::utils::collect_stream;
    use crate::agents::{
        AgentActor, AgentActorEvent, Context, GenericToolExecutor, Tool, ToolCall,
        ToolCallFunction, ToolDef, ToolExecutor, call_tools,
    };
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatChunk, ChatModel};
    use crate::providers::{DeepSeekProvider, LlamaCppProvider};
    use async_trait::async_trait;
    use futures::StreamExt;
    use futures::stream::BoxStream;
    use serde_json::{Value, json};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

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
                reasoning_content: None,
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
                    reasoning_content,
                    tool_calls,
                } => {
                    println!(
                        "[{}] Assistant: {} (tool_calls: {:?})",
                        i, content, tool_calls
                    );
                    if let Some(rc) = reasoning_content {
                        println!("    Reasoning: {}", rc);
                    }
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

    /// 此测试需要设置 DEEPSEEK_API_KEY 环境变量
    ///
    /// 注意：由于真实 LLM 行为不可预测，此测试主要验证：
    /// 1. playwright_cli 工具能够被正确注册和调用
    /// 2. Agent 能够完成执行（Completed 或 MaxIterations）
    #[tokio::test]
    async fn test_agent_actor_with_deepseek_and_playwright() {
        dotenv::dotenv().ok();

        println!("\n========== AgentActor DeepSeek + Playwright Test ==========\n");

        // 1. 创建 DeepSeek Provider
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                println!("跳过测试: 未设置 DEEPSEEK_API_KEY 环境变量");
                return;
            }
        };

        // 2. 创建 ChatModel
        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        if let Err(e) = model.set_active_model("deepseek-chat") {
            println!("设置活动模型失败: {}", e);
            return;
        }

        // 3. 创建工具执行器并注册真实的 Playwright 工具
        let mut executor = GenericToolExecutor::new();
        executor.register(PlaywrightCliTool::new());
        let executor = Arc::new(executor);

        // 4. 创建上下文 - 使用更明确的提示引导模型使用工具
        let mut context = Context::new();
        context.add_message(Message::system(
            r#"你是一个有用的助手。当用户需要访问网页或提取网页内容时, 你可以使用工具, 回答要简洁。"#,
        ));
        context.add_message(Message::user(
            "请帮我总结 https://www.peopleapp.com/column/30051629695-500007391518 网页的内容",
        ));

        println!(
            "[User] 请帮我总结 https://www.peopleapp.com/column/30051629695-500007391518 网页的内容\n"
        );

        // 5. 创建 AgentActor - 增加最大迭代次数
        let actor = AgentActor::new(Arc::new(model), executor, context).with_max_iterations(10);

        // 6. 启动 Actor
        let mut handle = actor.run_loop();

        // 7. 收集事件
        let mut events: Vec<AgentActorEvent> = Vec::new();
        let mut tool_calls_log: Vec<String> = Vec::new();

        while let Some(event) = handle.event_rx.recv().await {
            match &event {
                AgentActorEvent::Chunk(content) => {
                    // print!("{}", content);
                }
                AgentActorEvent::ReasoningChunk(content) => {
                    // print!("{}", content);
                }
                AgentActorEvent::ToolCalls(calls) => {
                    println!("\n[Event] ToolCalls: {} 个工具调用", calls.len());
                    for call in calls {
                        let tool_name = call.get_name();
                        let args = call.get_arguments();
                        println!("  → 工具: {}", tool_name);
                        println!("  → 参数: {}", args);
                        tool_calls_log.push(tool_name.clone());
                    }
                }
                AgentActorEvent::ToolResult {
                    call_id,
                    success,
                    output,
                } => {
                    println!("\n[Event] ToolResult: id={}, success={}", call_id, success);
                    if let Ok(json_value) = serde_json::from_str::<Value>(output) {
                        if let Some(stdout) = json_value.get("stdout").and_then(|v| v.as_str()) {
                            let preview = if stdout.len() > 150 {
                                format!("{}...", &stdout[..150])
                            } else {
                                stdout.to_string()
                            };
                            println!("  → 输出: {}", preview.replace('\n', " "));
                        }
                    }
                }
                AgentActorEvent::StepCompleted {
                    content,
                    reasoning_content,
                    tool_calls,
                } => {
                    if !content.is_empty() {
                        println!("\n[assistant-reasoning] {:?}", reasoning_content);
                        println!("\n[assistant] {:?}", content);
                        println!("\n[assistant-tool-calls] {:?}", tool_calls);
                    }
                }
                AgentActorEvent::Iteration { iteration, .. } => {
                    println!("\n--- Iteration {} ---", iteration);
                }
                AgentActorEvent::Completed => {
                    println!("\n\n[Event] Completed");
                }
                AgentActorEvent::Error(e) => {
                    println!("\n[Event] Error: {}", e);
                }
                AgentActorEvent::MaxIterations { iteration } => {
                    println!("\n[Event] MaxIterations: iteration={}", iteration);
                }
            }

            events.push(event.clone());

            if matches!(
                events.last(),
                Some(AgentActorEvent::Completed)
                    | Some(AgentActorEvent::Error(_))
                    | Some(AgentActorEvent::MaxIterations { .. })
            ) {
                break;
            }
        }

        // 8. 验证结果
        println!("\n========== 验证结果 ==========\n");

        // 验证执行结束（Completed 或 MaxIterations 都算成功）
        let finished = events.iter().any(|e| {
            matches!(
                e,
                AgentActorEvent::Completed | AgentActorEvent::MaxIterations { .. }
            )
        });
        assert!(finished, "Agent 应该正常结束执行");

        // 验证 playwright_cli 被调用（这是核心验证点）
        let playwright_calls = tool_calls_log
            .iter()
            .filter(|name| *name == "playwright_cli")
            .count();
        println!("playwright_cli 调用次数: {}", playwright_calls);
        assert!(playwright_calls >= 1, "应该至少调用一次 playwright_cli");

        println!("\n========== 测试通过 ==========\n");
    }
}
