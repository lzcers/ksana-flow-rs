#[cfg(test)]
mod tests {
    use std::pin::pin;
    use std::sync::Mutex;
    use std::time::Duration;

    use crate::agents::tools::playwright_cli::PlaywrightCliTool;
    use crate::agents::{
        AfterCallModel, AfterCallTools, AfterStep, AgentActor, AgentActorEvent, AgentError,
        AgentHook, BeforeCallModel, BeforeCallTools, CallModelEvent, Context,
        ContextPersistenceHook, ExecutionMetrics, GenericToolExecutor, HookError, HookOutcome,
        HookPhase, HookRegistry, IterationEventHook, LifecycleHook, ModelEventCtx, StepHookContext,
        StepResult, StepResultDraft, Tool, ToolCall, ToolCallFunction, ToolDef, ToolExecutor,
        call_model, call_tools,
    };
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatChunk, ChatError, ChatModel};
    use crate::providers::deepseek_provider_from_env;
    use async_trait::async_trait;
    use futures::{StreamExt, stream::BoxStream};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tokio::sync::mpsc;

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

    struct SlowTool {
        def: ToolDef,
        delay: Duration,
    }

    impl SlowTool {
        fn new(delay: Duration) -> Self {
            Self {
                def: ToolDef {
                    name: "slow_tool".to_string(),
                    description: "delays before returning".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                delay,
            }
        }
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn definition(&self) -> &ToolDef {
            &self.def
        }

        async fn execute(
            &self,
            _arguments: Value,
        ) -> Result<Value, crate::agents::ToolExecutorError> {
            tokio::time::sleep(self.delay).await;
            Ok(json!({"status": "ok"}))
        }
    }

    #[derive(Clone)]
    struct MockChatModel {
        chunks: Vec<ChatChunk>,
    }

    impl MockChatModel {
        fn new(chunks: Vec<ChatChunk>) -> Self {
            Self { chunks }
        }
    }

    #[derive(Clone)]
    struct DelayedChatModel {
        delay: Duration,
        chunks: Vec<ChatChunk>,
    }

    impl DelayedChatModel {
        fn new(delay: Duration, chunks: Vec<ChatChunk>) -> Self {
            Self { delay, chunks }
        }
    }

    #[async_trait]
    impl ChatCapability for MockChatModel {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, ChatError> {
            Ok(Message::assistant("unused"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
            Ok(futures::stream::iter(self.chunks.clone()).boxed())
        }
    }

    #[async_trait]
    impl ChatCapability for DelayedChatModel {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, ChatError> {
            Ok(Message::assistant("unused"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
            tokio::time::sleep(self.delay).await;
            Ok(futures::stream::iter(self.chunks.clone()).boxed())
        }
    }

    struct RecordingHook {
        phases: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingHook {
        fn new(phases: Arc<Mutex<Vec<String>>>) -> Self {
            Self { phases }
        }

        fn push(&self, value: impl Into<String>) {
            self.phases.lock().unwrap().push(value.into());
        }
    }

    #[async_trait]
    impl AgentHook for RecordingHook {
        fn name(&self) -> &'static str {
            "recording"
        }

        async fn before_step(
            &self,
            ctx: &mut StepHookContext<'_>,
        ) -> Result<HookOutcome, HookError> {
            ctx.scratchpad.insert("event_count", 0usize);
            self.push("before_step");
            Ok(HookOutcome::Continue)
        }

        async fn before_call_model(
            &self,
            _ctx: &mut StepHookContext<'_>,
            _input: &mut BeforeCallModel<'_>,
        ) -> Result<HookOutcome, HookError> {
            self.push("before_call_model");
            Ok(HookOutcome::Continue)
        }

        async fn on_model_event(
            &self,
            ctx: &mut StepHookContext<'_>,
            input: &ModelEventCtx<'_>,
        ) -> Result<HookOutcome, HookError> {
            if let Some(event_count) = ctx.scratchpad.get_mut::<usize>("event_count") {
                *event_count += 1;
            }

            let phase = match input.event {
                CallModelEvent::TextChunk(_) => "on_model_event:text",
                CallModelEvent::ReasoningChunk(_) => "on_model_event:reasoning",
                CallModelEvent::Completed { .. } => "on_model_event:completed",
                CallModelEvent::Error(_) => "on_model_event:error",
            };
            self.push(phase);
            Ok(HookOutcome::Continue)
        }

        async fn after_call_model(
            &self,
            _ctx: &mut StepHookContext<'_>,
            input: &mut AfterCallModel<'_>,
        ) -> Result<HookOutcome, HookError> {
            self.push(format!(
                "after_call_model:{}",
                input.output.tool_calls.len()
            ));
            Ok(HookOutcome::Continue)
        }

        async fn before_call_tools(
            &self,
            _ctx: &mut StepHookContext<'_>,
            input: &mut BeforeCallTools<'_>,
        ) -> Result<HookOutcome, HookError> {
            self.push(format!("before_call_tools:{}", input.tool_calls.len()));
            Ok(HookOutcome::Continue)
        }

        async fn after_call_tools(
            &self,
            _ctx: &mut StepHookContext<'_>,
            input: &mut AfterCallTools<'_>,
        ) -> Result<HookOutcome, HookError> {
            self.push(format!("after_call_tools:{}", input.tool_results.len()));
            Ok(HookOutcome::Continue)
        }

        async fn after_step(
            &self,
            ctx: &mut StepHookContext<'_>,
            _input: &mut AfterStep<'_>,
        ) -> Result<HookOutcome, HookError> {
            let event_count = ctx
                .scratchpad
                .get::<usize>("event_count")
                .copied()
                .unwrap_or_default();
            self.push(format!("after_step:{event_count}"));
            Ok(HookOutcome::Continue)
        }
    }

    struct FailingHook;

    #[async_trait]
    impl AgentHook for FailingHook {
        fn name(&self) -> &'static str {
            "failing"
        }

        async fn before_call_model(
            &self,
            _ctx: &mut StepHookContext<'_>,
            _input: &mut BeforeCallModel<'_>,
        ) -> Result<HookOutcome, HookError> {
            Err(HookError::new("forced failure"))
        }
    }

    struct TimeoutFinalizeHook {
        seen: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl AgentHook for TimeoutFinalizeHook {
        fn name(&self) -> &'static str {
            "timeout_finalize"
        }

        async fn before_step(
            &self,
            ctx: &mut StepHookContext<'_>,
        ) -> Result<HookOutcome, HookError> {
            ctx.scratchpad
                .insert("timeout_finalize.marker", "before-step".to_string());
            Ok(HookOutcome::Continue)
        }

        async fn after_step(
            &self,
            ctx: &mut StepHookContext<'_>,
            input: &mut AfterStep<'_>,
        ) -> Result<HookOutcome, HookError> {
            if matches!(input.result, StepResultDraft::Error(AgentError::Timeout)) {
                let marker = ctx
                    .scratchpad
                    .get::<String>("timeout_finalize.marker")
                    .cloned()
                    .unwrap_or_else(|| "missing".to_string());
                self.seen.lock().unwrap().push(marker);
            }
            Ok(HookOutcome::Continue)
        }
    }

    fn default_test_hooks() -> HookRegistry {
        HookRegistry::default()
    }

    fn weather_tool_call() -> ToolCall {
        ToolCall {
            id: "call_weather".to_string(),
            call_type: Some("function".to_string()),
            index: Some(0),
            function: Some(ToolCallFunction {
                name: "get_weather".to_string(),
                arguments: r#"{"city":"北京"}"#.to_string(),
            }),
            name: None,
            arguments: None,
        }
    }

    fn slow_tool_call() -> ToolCall {
        ToolCall {
            id: "call_slow".to_string(),
            call_type: Some("function".to_string()),
            index: Some(0),
            function: Some(ToolCallFunction {
                name: "slow_tool".to_string(),
                arguments: "{}".to_string(),
            }),
            name: None,
            arguments: None,
        }
    }

    fn metrics_snapshot(
        actor: &AgentActor<impl ChatCapability + Send + Sync, impl ToolExecutor + Send>,
    ) -> ExecutionMetrics {
        serde_json::from_value(
            actor
                .hook_snapshot("metrics")
                .expect("metrics snapshot should exist"),
        )
        .expect("metrics snapshot should deserialize")
    }

    #[tokio::test]
    async fn test_agent_actor_hooks_lifecycle_without_tools() {
        let model = MockChatModel::new(vec![
            ChatChunk {
                content: "Hello".to_string(),
                reasoning_content: String::new(),
                is_finished: false,
                finish_reason: None,
                tool_calls: None,
            },
            ChatChunk {
                content: " world".to_string(),
                reasoning_content: "think".to_string(),
                is_finished: true,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
            },
        ]);

        let executor = GenericToolExecutor::new();
        let mut context = Context::new();
        context.add_message(Message::system("sys"));
        context.add_message(Message::user("hello"));

        let phases = Arc::new(Mutex::new(Vec::new()));
        let hooks = default_test_hooks().register(RecordingHook::new(phases.clone()));
        let mut actor = AgentActor::with_hooks(model, executor, context, hooks);

        let result = actor.run_step(None).await;

        match result {
            StepResult::Done {
                content,
                reasoning_content,
            } => {
                assert_eq!(content, "Hello world");
                assert_eq!(reasoning_content.as_deref(), Some("think"));
            }
            other => panic!("unexpected result: {other:?}"),
        }

        let phases = phases.lock().unwrap().clone();
        assert_eq!(
            phases,
            vec![
                "before_step",
                "before_call_model",
                "on_model_event:text",
                "on_model_event:text",
                "on_model_event:reasoning",
                "on_model_event:completed",
                "after_call_model:0",
                "after_step:4",
            ]
        );

        assert_eq!(actor.state().iteration, 1);
        assert_eq!(metrics_snapshot(&actor).iterations, 1);
        assert_eq!(actor.state().state, crate::agents::JobState::Completed);

        let conversation = actor.context().conversation();
        assert_eq!(conversation.len(), 3);
        match conversation.last().unwrap() {
            Message::Assistant {
                content,
                reasoning_content,
                tool_calls,
            } => {
                assert_eq!(content, "Hello world");
                assert_eq!(reasoning_content.as_deref(), Some("think"));
                assert!(tool_calls.is_none());
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_agent_actor_hooks_tool_phases_and_events() {
        let model = MockChatModel::new(vec![ChatChunk {
            content: "checking weather".to_string(),
            reasoning_content: String::new(),
            is_finished: true,
            finish_reason: Some("tool_calls".to_string()),
            tool_calls: Some(vec![weather_tool_call()]),
        }]);

        let mut executor = GenericToolExecutor::new();
        executor.register(MockWeatherTool::new());

        let mut context = Context::new();
        context.add_message(Message::system("sys"));
        context.add_message(Message::user("weather"));

        let phases = Arc::new(Mutex::new(Vec::new()));
        let hooks = default_test_hooks().register(RecordingHook::new(phases.clone()));
        let mut actor = AgentActor::with_hooks(model, executor, context, hooks);
        let (event_tx, mut event_rx) = mpsc::channel(16);

        let result = actor.run_step(Some(event_tx)).await;

        match result {
            StepResult::Continue {
                content,
                tool_calls,
                tool_results,
                ..
            } => {
                assert_eq!(content, "checking weather");
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_results.len(), 1);
                assert!(tool_results[0].success);
                assert!(tool_results[0].output.contains("北京"));
            }
            other => panic!("unexpected result: {other:?}"),
        }

        let phases = phases.lock().unwrap().clone();
        assert_eq!(
            phases,
            vec![
                "before_step",
                "before_call_model",
                "on_model_event:text",
                "on_model_event:completed",
                "after_call_model:1",
                "before_call_tools:1",
                "after_call_tools:1",
                "after_step:2",
            ]
        );

        assert_eq!(actor.state().state, crate::agents::JobState::WaitingInput);
        assert_eq!(metrics_snapshot(&actor).tool_calls_count, 1);

        let conversation = actor.context().conversation();
        assert_eq!(conversation.len(), 4);
        match &conversation[2] {
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                assert_eq!(content, "checking weather");
                assert_eq!(tool_calls.as_ref().map(Vec::len), Some(1));
            }
            other => panic!("unexpected assistant message: {other:?}"),
        }
        match &conversation[3] {
            Message::Tool {
                tool_call_id,
                content,
            } => {
                assert_eq!(tool_call_id, "call_weather");
                assert!(content.contains("北京"));
            }
            other => panic!("unexpected tool message: {other:?}"),
        }

        let mut saw_step_completed = false;
        let mut saw_tool_calls = false;
        let mut saw_tool_result = false;
        let mut saw_iteration = false;
        while let Ok(event) = event_rx.try_recv() {
            match event {
                AgentActorEvent::StepCompleted { .. } => saw_step_completed = true,
                AgentActorEvent::ToolCalls(calls) => {
                    saw_tool_calls = true;
                    assert_eq!(calls.len(), 1);
                }
                AgentActorEvent::ToolResult { success, .. } => {
                    saw_tool_result = true;
                    assert!(success);
                }
                AgentActorEvent::Iteration { iteration, .. } => {
                    saw_iteration = true;
                    assert_eq!(iteration, 1);
                }
                AgentActorEvent::ContentChunk(_)
                | AgentActorEvent::ReasoningChunk(_)
                | AgentActorEvent::Completed
                | AgentActorEvent::Cancelled
                | AgentActorEvent::Error(_)
                | AgentActorEvent::MaxIterations { .. } => {}
            }
        }
        assert!(saw_step_completed);
        assert!(saw_tool_calls);
        assert!(saw_tool_result);
        assert!(saw_iteration);
    }

    #[tokio::test]
    async fn test_agent_actor_hook_failure_transitions_state() {
        let model = MockChatModel::new(vec![ChatChunk {
            content: "unused".to_string(),
            reasoning_content: String::new(),
            is_finished: true,
            finish_reason: Some("stop".to_string()),
            tool_calls: None,
        }]);
        let executor = GenericToolExecutor::new();

        let mut context = Context::new();
        context.add_message(Message::user("hello"));

        let hooks = HookRegistry::empty()
            .register(LifecycleHook)
            .register(FailingHook)
            .register(ContextPersistenceHook)
            .register(IterationEventHook);
        let mut actor = AgentActor::with_hooks(model, executor, context, hooks);

        let result = actor.run_step(None).await;

        match result {
            StepResult::Error(AgentError::Hook {
                plugin,
                phase,
                message,
            }) => {
                assert_eq!(plugin, "failing");
                assert_eq!(phase, HookPhase::BeforeCallModel);
                assert_eq!(message, "forced failure");
            }
            other => panic!("unexpected result: {other:?}"),
        }

        assert_eq!(actor.state().state, crate::agents::JobState::Failed);
        assert_eq!(actor.context().conversation().len(), 1);
    }

    #[tokio::test]
    async fn test_agent_actor_hook_step_timeout_policy_and_snapshots() {
        let model = DelayedChatModel::new(
            Duration::from_millis(30),
            vec![ChatChunk {
                content: "late".to_string(),
                reasoning_content: String::new(),
                is_finished: true,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
            }],
        );
        let executor = GenericToolExecutor::new();

        let mut context = Context::new();
        context.add_message(Message::user("hello"));

        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut actor = crate::agents::AgentActorBuilder::new(model, executor)
            .context(context)
            .hook(TimeoutFinalizeHook { seen: seen.clone() })
            .step_timeout(Duration::from_millis(5))
            .build();

        let result = actor.run_step(None).await;

        match result {
            StepResult::Error(AgentError::Timeout) => {}
            other => panic!("unexpected result: {other:?}"),
        }

        assert_eq!(actor.state().state, crate::agents::JobState::Failed);
        assert_eq!(
            actor.hook_snapshot("timeout_policy"),
            Some(json!({
                "step_timeout_ms": 5_u64,
                "tool_timeout_ms": Value::Null,
            }))
        );

        let metrics = metrics_snapshot(&actor);
        assert_eq!(metrics.iterations, 1);
        assert!(metrics.total_duration >= Duration::from_millis(5));
        assert_eq!(seen.lock().unwrap().as_slice(), ["before-step"]);

        let snapshots = actor.hook_snapshots();
        assert!(snapshots.contains_key("metrics"));
        assert!(snapshots.contains_key("timeout_policy"));
    }

    #[tokio::test]
    async fn test_agent_actor_hook_tool_timeout_policy_execution() {
        let model = MockChatModel::new(vec![ChatChunk {
            content: "run slow tool".to_string(),
            reasoning_content: String::new(),
            is_finished: true,
            finish_reason: Some("tool_calls".to_string()),
            tool_calls: Some(vec![slow_tool_call()]),
        }]);

        let mut executor = GenericToolExecutor::new();
        executor.register(SlowTool::new(Duration::from_millis(30)));

        let mut context = Context::new();
        context.add_message(Message::user("slow tool"));

        let mut actor = crate::agents::AgentActorBuilder::new(model, executor)
            .context(context)
            .tool_timeout(Duration::from_millis(5))
            .build();

        let result = actor.run_step(None).await;

        match result {
            StepResult::Error(AgentError::Timeout) => {}
            other => panic!("unexpected result: {other:?}"),
        }

        assert_eq!(actor.state().state, crate::agents::JobState::Failed);
        assert_eq!(
            actor.hook_snapshot("timeout_policy"),
            Some(json!({
                "step_timeout_ms": Value::Null,
                "tool_timeout_ms": 5_u64,
            }))
        );
    }

    /// 测试 Agent Loop - 流式工具调用流程
    /// 目的：展示 Agent 流式调用工具的完整过程
    #[tokio::test]
    async fn test_agent_loop_with_tools() {
        dotenv::dotenv().ok();

        // 1. 创建 ChatModel
        let provider = match deepseek_provider_from_env() {
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
            let stream = call_model(model.as_ref(), &messages_clone, Some(executor.tools()));
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
        let provider = match deepseek_provider_from_env() {
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

        // 4. 创建上下文 - 使用更明确的提示引导模型使用工具
        let mut context = Context::new();
        context.add_message(Message::system(
            r#"你是一个聪明的助手，当用户需要访问网页或提取网页内容时, 你可以使用工具完成任务。"#,
        ));
        context.add_message(Message::user(
            "请帮我总结 https://www.peopleapp.com/column/30051629695-500007391518 网页的内容",
        ));

        println!(
            "[User] 请帮我总结 https://www.peopleapp.com/column/30051629695-500007391518 网页的内容\n"
        );

        // 5. 创建 AgentActo
        let actor = AgentActor::new(model, executor, context);

        // 6. 启动 Actor
        let mut handle = actor.run_loop();

        // 7. 收集事件
        let mut events: Vec<AgentActorEvent> = Vec::new();
        let mut tool_calls_log: Vec<String> = Vec::new();

        while let Some(event) = handle.event_rx.recv().await {
            match &event {
                AgentActorEvent::ContentChunk(_) => {
                    // print!("{}", content);
                }
                AgentActorEvent::ReasoningChunk(_) => {
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
                AgentActorEvent::Cancelled => {
                    println!("\n\n[Event] Cancelled");
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
                    | Some(AgentActorEvent::Cancelled)
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
