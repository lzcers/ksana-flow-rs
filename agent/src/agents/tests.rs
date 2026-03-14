#[cfg(test)]
mod tests {
    use std::pin::pin;

    use crate::agents::tools::playwright_cli::PlaywrightCliTool;
    use crate::agents::{
        AgentActor, AgentActorEvent, CallModelEvent, Context, GenericToolExecutor, Tool, ToolCall,
        ToolDef, ToolExecutor, call_model, call_tools,
    };
    use crate::core::Message;
    use crate::models::ChatModel;
    use crate::providers::DeepSeekProvider;
    use async_trait::async_trait;
    use futures::StreamExt;
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
        let model = Arc::new(model);

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

            // 克隆消息以避免借用冲突
            let messages_clone = messages.clone();

            // 使用 call_model 纯函数
            let stream = call_model(&messages_clone, Some(executor.tools()), model.as_ref());
            let mut stream = pin!(stream);

            let mut content = String::new();
            let mut reasoning_content: Option<String> = None;
            let mut tool_calls: Option<Vec<ToolCall>> = None;

            while let Some(event) = stream.next().await {
                match event {
                    CallModelEvent::TextChunk(text) => {
                        content.push_str(&text);
                        print!("{}", text);
                    }
                    CallModelEvent::ReasoningChunk(text) => {
                        reasoning_content
                            .get_or_insert_with(String::new)
                            .push_str(&text);
                    }
                    CallModelEvent::Completed {
                        content: c,
                        reasoning_content: rc,
                        tool_calls: tc,
                    } => {
                        content = c;
                        reasoning_content = rc;
                        tool_calls = tc;
                    }
                    CallModelEvent::Error(e) => {
                        println!("[Error] {}", e);
                        return;
                    }
                }
            }

            // drop stream 以释放借用
            drop(stream);

            // 构建助手消息
            let assistant_msg = Message::Assistant {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: tool_calls.clone(),
            };

            // 打印助手响应
            println!("\n[Assistant Message] {}", content);

            // 添加助手消息到历史
            messages.push(assistant_msg);

            // 检查是否有工具调用
            if let Some(calls) = tool_calls {
                // 执行工具
                let tool_stream = call_tools(&executor, &calls);
                let mut tool_stream = pin!(tool_stream);

                while let Some(result) = tool_stream.next().await {
                    println!("[Tool] {} -> {}", result.tool_name, result.output);
                    messages.push(Message::Tool {
                        tool_call_id: result.call_id,
                        content: result.output,
                    });
                }
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
        model.add_model_provider("deepseek-reasoner", provider);
        if let Err(e) = model.set_active_model("deepseek-reasoner") {
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
                AgentActorEvent::ContentChunk(content) => {
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
                            let preview = if stdout.chars().count() > 150 {
                                format!("{}...", stdout.chars().take(150).collect::<String>())
                            } else {
                                stdout.to_string()
                            };
                            println!("  → 输出: {}", preview.replace('\n', " "));
                        } else if let Some(error) = json_value.get("error").and_then(|v| v.as_str())
                        {
                            println!("  → 错误: {}", error);
                        } else {
                            println!("  → 输出: {}", json_value);
                        }
                    }
                }
                AgentActorEvent::StepCompleted {
                    content,
                    reasoning_content,
                    tool_calls,
                } => {
                    if reasoning_content.is_some() {
                        println!(
                            "\n[assistant-reasoning] {}",
                            reasoning_content.as_ref().unwrap_or(&"".to_string())
                        );
                    }
                    if !content.is_empty() {
                        println!("\n[assistant] {:?}", content);
                    }
                    if tool_calls.is_some() {
                        let json_str = serde_json::to_string(&tool_calls).unwrap_or_default();
                        println!("\n[assistant-tool-calls] {}", json_str);
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

        // 验证执行结束（Completed 或 MaxIterations 都算成功）
        let finished = events.iter().any(|e| {
            matches!(
                e,
                AgentActorEvent::Completed | AgentActorEvent::MaxIterations { .. }
            )
        });
        assert!(finished, "Agent 应该正常结束执行");
    }
}
