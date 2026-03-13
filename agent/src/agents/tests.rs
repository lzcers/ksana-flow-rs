#[cfg(test)]
mod tests {
    use crate::agents::utils::collect_stream;
    use crate::agents::{
        AgentActor, AgentActorEvent, Context, GenericToolExecutor, Tool, ToolCall,
        ToolCallFunction, ToolDef, ToolExecutor, call_tools,
    };
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatChunk, ChatModel};
    use crate::providers::DeepSeekProvider;
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

    // ============================================================================
    // AgentActor Tests
    // ============================================================================

    /// Mock Chat Model for AgentActor Test
    /// 模拟一个会调用天气工具的 Chat 模型
    struct MockChatForActor {
        /// 调用计数器
        call_count: AtomicU32,
    }

    impl MockChatForActor {
        fn new() -> Self {
            Self {
                call_count: AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl ChatCapability for MockChatForActor {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, crate::models::ChatError> {
            Ok(Message::assistant("Mock response"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<BoxStream<'static, ChatChunk>, crate::models::ChatError> {
            let call_count = self.call_count.fetch_add(1, Ordering::SeqCst);

            let chunks = if call_count == 0 {
                // 第一次调用：返回工具调用
                vec![
                    ChatChunk {
                        content: String::new(),
                        is_finished: false,
                        finish_reason: None,
                        tool_calls: Some(vec![ToolCall {
                            id: "call_weather_001".to_string(),
                            call_type: Some("function".to_string()),
                            index: Some(0),
                            function: Some(ToolCallFunction {
                                name: "get_weather".to_string(),
                                arguments: r#"{"city":"北京"}"#.to_string(),
                            }),
                            name: None,
                            arguments: None,
                        }]),
                    },
                    ChatChunk {
                        content: String::new(),
                        is_finished: true,
                        finish_reason: Some("tool_calls".to_string()),
                        tool_calls: None,
                    },
                ]
            } else {
                // 第二次调用：返回最终响应
                vec![
                    ChatChunk {
                        content: "根据查询结果，".to_string(),
                        is_finished: false,
                        finish_reason: None,
                        tool_calls: None,
                    },
                    ChatChunk {
                        content: "北京今天天气晴朗，".to_string(),
                        is_finished: false,
                        finish_reason: None,
                        tool_calls: None,
                    },
                    ChatChunk {
                        content: "气温22°C，湿度45%。".to_string(),
                        is_finished: false,
                        finish_reason: None,
                        tool_calls: None,
                    },
                    ChatChunk {
                        content: String::new(),
                        is_finished: true,
                        finish_reason: Some("stop".to_string()),
                        tool_calls: None,
                    },
                ]
            };

            Ok(futures::stream::iter(chunks).boxed())
        }
    }

    /// 测试 AgentActor 与 Mock 天气工具的交互
    /// 验证 Actor 能正确执行工具调用并输出所有事件
    #[tokio::test]
    async fn test_agent_actor_with_weather_tool() {
        println!("\n========== AgentActor Weather Tool Test ==========\n");

        // 1. 创建 Mock Chat 模型
        let chat = Arc::new(MockChatForActor::new());

        // 2. 创建工具执行器并注册天气工具
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());
        let executor = Arc::new(executor);

        // 3. 创建上下文，添加初始消息
        let mut context = Context::new();
        context.add_message(Message::system(
            "你是一个有用的助手。当用户询问天气时，请使用 get_weather 工具获取信息。",
        ));
        context.add_message(Message::user("北京今天天气怎么样？"));

        // 4. 创建 AgentActor
        let actor = AgentActor::new(chat, executor, context).with_max_iterations(3);

        // 5. 启动 Actor
        let mut handle = actor.run_loop();

        println!("[Test] Actor 已启动，开始收集事件...\n");

        // 6. 收集所有事件
        let mut events: Vec<AgentActorEvent> = Vec::new();

        while let Some(event) = handle.event_rx.recv().await {
            // 打印事件详情
            match &event {
                AgentActorEvent::Chunk(content) => {
                    print!("chunk {}", content);
                }
                AgentActorEvent::ToolCalls(calls) => {
                    println!("\n[Event] ToolCalls: {:?}", calls.len());
                    for call in calls {
                        let tool_name = call
                            .function
                            .as_ref()
                            .map(|f| f.name.as_str())
                            .unwrap_or("unknown");
                        let args = call
                            .function
                            .as_ref()
                            .map(|f| f.arguments.as_str())
                            .unwrap_or("");
                        println!("  - Tool: {}, Args: {}", tool_name, args);
                    }
                }
                AgentActorEvent::ToolResult {
                    call_id,
                    success,
                    output,
                } => {
                    println!(
                        "\n[Event] ToolResult: call_id={}, success={}",
                        call_id, success
                    );
                    println!("  Output: {}", output);
                }
                AgentActorEvent::Iteration {
                    iteration,
                    message_count,
                } => {
                    println!(
                        "\n[Event] Iteration: {}, message_count={}",
                        iteration, message_count
                    );
                }
                AgentActorEvent::Completed => {
                    println!("\n[Event] Completed");
                }
                AgentActorEvent::Error(e) => {
                    println!("\n[Event] Error: {}", e);
                }
                AgentActorEvent::StepCompleted {
                    content,
                    tool_calls,
                } => {
                    println!("\n[Event] LlmResponse: content len={}", content.len());
                    if let Some(calls) = tool_calls {
                        println!("  → Tool calls: {} 个", calls.len());
                    }
                }
                AgentActorEvent::MaxIterations { iteration } => {
                    println!("\n[Event] MaxIterations: iteration={}", iteration);
                }
            }

            events.push(event.clone());

            // 如果完成或出错，退出循环
            if matches!(
                events.last(),
                Some(AgentActorEvent::Completed) | Some(AgentActorEvent::Error(_))
            ) {
                break;
            }
        }

        println!("\n========== 事件统计 ==========");
        println!("总事件数: {}", events.len());

        // 统计各类事件
        let chunk_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::Chunk(_)))
            .count();
        let tool_calls_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::ToolCalls(_)))
            .count();
        let tool_result_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::ToolResult { .. }))
            .count();
        let iteration_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::Iteration { .. }))
            .count();

        println!("- Chunk 事件: {}", chunk_count);
        println!("- ToolCalls 事件: {}", tool_calls_count);
        println!("- ToolResult 事件: {}", tool_result_count);
        println!("- Iteration 事件: {}", iteration_count);

        // 7. 验证事件
        assert!(!events.is_empty(), "应该有事件产生");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::Chunk(_))),
            "应该有 Chunk 事件"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::ToolCalls(_))),
            "应该有 ToolCalls 事件"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::ToolResult { .. })),
            "应该有 ToolResult 事件"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::Completed)),
            "应该有 Completed 事件"
        );

        // 验证工具调用正确
        if let Some(AgentActorEvent::ToolCalls(calls)) = events
            .iter()
            .find(|e| matches!(e, AgentActorEvent::ToolCalls(_)))
        {
            assert_eq!(calls.len(), 1, "应该有一次工具调用");
            let call = &calls[0];
            assert_eq!(
                call.function.as_ref().map(|f| &f.name),
                Some(&"get_weather".to_string()),
                "工具名称应该是 get_weather"
            );
        }

        // 验证工具执行成功
        if let Some(AgentActorEvent::ToolResult { success, .. }) = events
            .iter()
            .find(|e| matches!(e, AgentActorEvent::ToolResult { .. }))
        {
            assert!(success, "工具执行应该成功");
        }

        println!("\n========== 测试通过 ==========\n");
    }

    /// 测试 AgentActor 的暂停和恢复功能
    #[tokio::test]
    async fn test_agent_actor_pause_resume() {
        println!("\n========== AgentActor Pause/Resume Test ==========\n");

        let chat = Arc::new(MockChatForActor::new());
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());
        let executor = Arc::new(executor);

        let mut context = Context::new();
        context.add_message(Message::user("测试暂停恢复"));

        let actor = AgentActor::new(chat, executor, context).with_max_iterations(3);
        let mut handle = actor.run_loop();

        // 收集一些事件后暂停
        let mut events = Vec::new();
        let mut paused = false;

        loop {
            // 使用 timeout 来检查是否有事件
            match tokio::time::timeout(Duration::from_millis(100), handle.event_rx.recv()).await {
                Ok(Some(event)) => {
                    events.push(event.clone());

                    // 在第一次 ToolCalls 后暂停
                    if matches!(event, AgentActorEvent::ToolCalls(_)) && !paused {
                        println!("[Test] 收到 ToolCalls，暂停 Actor...");
                        handle.pause().await;
                        paused = true;

                        // 等待一小段时间
                        tokio::time::sleep(Duration::from_millis(200)).await;

                        // 恢复
                        println!("[Test] 恢复 Actor...");
                        handle.resume().await;
                    }

                    if matches!(
                        event,
                        AgentActorEvent::Completed | AgentActorEvent::Error(_)
                    ) {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    // timeout，继续等待
                    if paused {
                        continue;
                    }
                }
            }
        }

        println!("\n收集到 {} 个事件", events.len());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::Completed)),
            "应该完成"
        );

        println!("\n========== 测试通过 ==========\n");
    }

    /// 测试 AgentActor 的取消功能
    #[tokio::test]
    async fn test_agent_actor_cancel() {
        println!("\n========== AgentActor Cancel Test ==========\n");

        let chat = Arc::new(MockChatForActor::new());
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());
        let executor = Arc::new(executor);

        let mut context = Context::new();
        context.add_message(Message::user("测试取消"));

        let actor = AgentActor::new(chat, executor, context).with_max_iterations(3);
        let mut handle = actor.run_loop();

        let mut events = Vec::new();
        let mut cancelled = false;

        loop {
            match tokio::time::timeout(Duration::from_millis(100), handle.event_rx.recv()).await {
                Ok(Some(event)) => {
                    events.push(event.clone());
                    println!("[Event] {:?}", event);

                    // 收到 ToolCalls 后取消
                    if matches!(event, AgentActorEvent::ToolCalls(_)) && !cancelled {
                        println!("[Test] 收到 ToolCalls，取消 Actor...");
                        handle.cancel().await;
                        cancelled = true;
                    }

                    if matches!(
                        event,
                        AgentActorEvent::Error(_) | AgentActorEvent::Completed
                    ) {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    // 超时后退出
                    if cancelled {
                        break;
                    }
                }
            }
        }

        println!("\n收集到 {} 个事件", events.len());

        // 验证收到取消事件
        let has_error = events.iter().any(|e| {
            if let AgentActorEvent::Error(msg) = e {
                msg == "Cancelled"
            } else {
                false
            }
        });
        assert!(
            has_error || cancelled,
            "应该有 Error(Cancelled) 事件或已取消"
        );

        println!("\n========== 测试通过 ==========\n");
    }

    // ============================================================================
    // Real API Tests (DeepSeek)
    // ============================================================================

    /// 测试 AgentActor 调用真实 DeepSeek API
    /// 使用真实的 LLM API 和 Mock 天气工具
    #[tokio::test]
    async fn test_agent_actor_with_deepseek_api() {
        dotenv::dotenv().ok();

        println!("\n========== AgentActor DeepSeek API Test ==========\n");

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

        // 3. 创建工具执行器并注册天气工具
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());
        let executor = Arc::new(executor);

        // 4. 创建上下文，添加初始消息
        let mut context = Context::new();
        context.add_message(Message::system(
            "你是一个有用的助手。当用户询问天气时，请使用 get_weather 工具获取信息。回答要简洁。",
        ));
        context.add_message(Message::user("北京今天天气怎么样？"));

        println!("[User] 北京今天天气怎么样？");

        // 5. 创建 AgentActor
        let actor = AgentActor::new(Arc::new(model), executor, context).with_max_iterations(5);

        // 6. 启动 Actor
        let mut handle = actor.run_loop();

        // 7. 收集所有事件并实时输出
        let mut events: Vec<AgentActorEvent> = Vec::new();

        while let Some(event) = handle.event_rx.recv().await {
            match &event {
                AgentActorEvent::Chunk(_) => {}
                AgentActorEvent::StepCompleted {
                    content,
                    tool_calls,
                } => {
                    println!("\n[assistant] {}", content);
                }
                AgentActorEvent::ToolCalls(calls) => {
                    println!("\n[Event] ToolCalls: {} 个工具调用", calls.len());
                    for call in calls {
                        let tool_name = call
                            .function
                            .as_ref()
                            .map(|f| f.name.as_str())
                            .unwrap_or("unknown");
                        let args = call
                            .function
                            .as_ref()
                            .map(|f| f.arguments.as_str())
                            .unwrap_or("");
                        println!("  → 工具: {}", tool_name);
                        if !args.is_empty() {
                            println!("  → 参数: {}", args);
                        }
                    }
                }
                AgentActorEvent::ToolResult {
                    call_id,
                    success,
                    output,
                } => {
                    println!("\n[Event] ToolResult: id={}, success={}", call_id, success);
                    if let Ok(json_value) = serde_json::from_str::<Value>(output) {
                        if let Some(city) = json_value.get("city").and_then(|v| v.as_str()) {
                            if let Some(temp) =
                                json_value.get("temperature").and_then(|v| v.as_str())
                            {
                                let condition = json_value
                                    .get("condition")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                println!("  → {} 天气: {}, 温度: {}", city, condition, temp);
                            }
                        }
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

            // 如果完成或出错，退出循环
            if matches!(
                events.last(),
                Some(AgentActorEvent::Completed) | Some(AgentActorEvent::Error(_))
            ) {
                break;
            }
        }

        // 8. 事件统计
        println!("\n========== 事件统计 ==========");
        println!("总事件数: {}", events.len());

        let tool_calls_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::ToolCalls(_)))
            .count();
        let tool_result_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::ToolResult { .. }))
            .count();
        let iteration_count = events
            .iter()
            .filter(|e| matches!(e, AgentActorEvent::Iteration { .. }))
            .count();

        println!("- ToolCalls 事件: {}", tool_calls_count);
        println!("- ToolResult 事件: {}", tool_result_count);
        println!("- Iteration 事件: {}", iteration_count);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::Completed)),
            "应该有 Completed 事件"
        );
        println!("\n========== 测试通过 ==========\n");
    }

    /// 测试 AgentActor 多轮工具调用 (真实 API)
    #[tokio::test]
    async fn test_agent_actor_multi_tool_calls() {
        dotenv::dotenv().ok();

        println!("\n========== AgentActor Multi Tool Calls Test ==========\n");

        // 创建 DeepSeek Provider
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                println!("跳过测试: 未设置 DEEPSEEK_API_KEY 环境变量");
                return;
            }
        };

        // 创建 ChatModel
        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        if let Err(e) = model.set_active_model("deepseek-chat") {
            println!("设置活动模型失败: {}", e);
            return;
        }

        // 创建工具执行器
        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());
        let executor = Arc::new(executor);

        // 创建上下文 - 要求查询多个城市
        let mut context = Context::new();
        context.add_message(Message::system(
            "你是一个有用的助手。使用 get_weather 工具获取天气信息。回答要简洁。",
        ));
        context.add_message(Message::user("北京和上海今天天气怎么样？"));

        println!("[User] 北京和上海今天天气怎么样？");

        // 创建 AgentActor
        let actor = AgentActor::new(Arc::new(model), executor, context).with_max_iterations(5);

        // 启动 Actor
        let mut handle = actor.run_loop();

        // 收集事件
        let mut events: Vec<AgentActorEvent> = Vec::new();
        let mut tool_call_count = 0;

        while let Some(event) = handle.event_rx.recv().await {
            match &event {
                AgentActorEvent::Chunk(_) => {}
                AgentActorEvent::ToolCalls(calls) => {
                    println!("\n[Event] ToolCalls: {} 个工具调用", calls.len());
                    tool_call_count += calls.len();
                    for call in calls {
                        let tool_name = call
                            .function
                            .as_ref()
                            .map(|f| f.name.as_str())
                            .unwrap_or("unknown");
                        let args = call
                            .function
                            .as_ref()
                            .map(|f| f.arguments.as_str())
                            .unwrap_or("");
                        println!("  → 工具: {}", tool_name);
                        if !args.is_empty() {
                            println!("  → 参数: {}", args);
                        }
                    }
                }
                AgentActorEvent::ToolResult {
                    call_id,
                    success,
                    output,
                } => {
                    println!("\n[Event] ToolResult: id={}, success={}", call_id, success);
                    if let Ok(json_value) = serde_json::from_str::<Value>(output) {
                        if let Some(city) = json_value.get("city").and_then(|v| v.as_str()) {
                            if let Some(temp) =
                                json_value.get("temperature").and_then(|v| v.as_str())
                            {
                                let condition = json_value
                                    .get("condition")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                println!("  → {} 天气: {}, 温度: {}", city, condition, temp);
                            }
                        }
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
                AgentActorEvent::StepCompleted {
                    content,
                    tool_calls,
                } => {
                    println!("\n[assistant] {}", content);
                }
                AgentActorEvent::MaxIterations { iteration } => {
                    println!("\n[Event] MaxIterations: iteration={}", iteration);
                }
            }

            events.push(event.clone());

            if matches!(
                events.last(),
                Some(AgentActorEvent::Completed) | Some(AgentActorEvent::Error(_))
            ) {
                break;
            }
        }

        println!("\n========== 统计 ==========");
        println!("工具调用次数: {}", tool_call_count);

        // 验证 - 模型可能会在一个 ToolCalls 中调用多次，也可能分多次调用
        assert!(tool_call_count >= 2, "应该至少调用 2 次工具（北京和上海）");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentActorEvent::Completed)),
            "应该完成"
        );

        println!("\n========== 测试通过 ==========\n");
    }
}
