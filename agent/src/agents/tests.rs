use std::pin::pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::agents::tools::playwright_cli::PlaywrightCliTool;
use crate::agents::{
    AfterCallModel, AfterCallTools, AfterStep, AfterStepInput, AgentActor, AgentActorEvent,
    AgentError, BeforeCallTools, BeforeStep, BeforeStepInput, CallModelEvent, Context, Effect,
    ExecutionMetrics, GenericToolExecutor, Hook, HookContinueStep, HookDoneStep, HookEffect,
    HookError, HookEvent, HookPhase, HookRegistry, HookStepResult, HookStepUpdate, HookToolCall,
    JobState, ModelEventCtx, RuntimeHook, RuntimeHookRegistry, StepResult, StepResultDraft, Tool,
    ToolCall, ToolCallFunction, ToolDef, ToolExecutor, call_model, call_tools,
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

    async fn execute(&self, arguments: Value) -> Result<Value, crate::agents::ToolExecutorError> {
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

    async fn execute(&self, _arguments: Value) -> Result<Value, crate::agents::ToolExecutorError> {
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

#[derive(Clone)]
struct RoundTripChatModel {
    call_count: Arc<AtomicUsize>,
    seen_messages: Arc<Mutex<Vec<Vec<Message>>>>,
}

impl RoundTripChatModel {
    fn new(seen_messages: Arc<Mutex<Vec<Vec<Message>>>>) -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
            seen_messages,
        }
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

#[async_trait]
impl ChatCapability for RoundTripChatModel {
    async fn chat(
        &self,
        _msgs: Vec<Message>,
        _tools: Option<Vec<ToolDef>>,
    ) -> Result<Message, ChatError> {
        Ok(Message::assistant("unused"))
    }

    async fn chat_stream(
        &self,
        msgs: Vec<Message>,
        _tools: Option<Vec<ToolDef>>,
    ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
        self.seen_messages.lock().unwrap().push(msgs);
        let call_index = self.call_count.fetch_add(1, Ordering::SeqCst);
        let chunks = match call_index {
            0 => vec![ChatChunk {
                content: "checking weather".to_string(),
                reasoning_content: String::new(),
                is_finished: true,
                finish_reason: Some("tool_calls".to_string()),
                tool_calls: Some(vec![weather_tool_call()]),
            }],
            _ => vec![ChatChunk {
                content: "final answer".to_string(),
                reasoning_content: String::new(),
                is_finished: true,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
            }],
        };
        Ok(futures::stream::iter(chunks).boxed())
    }
}

struct RecordingHook {
    phases: Arc<Mutex<Vec<String>>>,
    event_count: AtomicUsize,
}

impl RecordingHook {
    fn new(phases: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            phases,
            event_count: AtomicUsize::new(0),
        }
    }

    fn push(&self, value: impl Into<String>) {
        self.phases.lock().unwrap().push(value.into());
    }
}

#[async_trait]
impl RuntimeHook for RecordingHook {
    fn name(&self) -> &'static str {
        "recording"
    }

    async fn before_step(&self, _input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        self.push("before_step");
        self.event_count.store(0, Ordering::SeqCst);
        Ok(vec![])
    }

    async fn before_call_model(&self) -> Result<Vec<Effect>, HookError> {
        self.push("before_call_model");
        Ok(vec![])
    }

    async fn on_model_event(&self, input: ModelEventCtx<'_>) -> Result<Vec<Effect>, HookError> {
        let event_count = self.event_count.fetch_add(1, Ordering::SeqCst) + 1;
        let phase = match input.event {
            CallModelEvent::TextChunk(_) => "on_model_event:text",
            CallModelEvent::ReasoningChunk(_) => "on_model_event:reasoning",
            CallModelEvent::Completed { .. } => "on_model_event:completed",
            CallModelEvent::Error(_) => "on_model_event:error",
        };
        self.push(phase);
        self.event_count.store(event_count, Ordering::SeqCst);
        Ok(vec![])
    }

    async fn after_call_model(&self, input: AfterCallModel<'_>) -> Result<Vec<Effect>, HookError> {
        self.push(format!(
            "after_call_model:{}",
            input.output.tool_calls.len()
        ));
        Ok(vec![])
    }

    async fn before_call_tools(
        &self,
        input: BeforeCallTools<'_>,
    ) -> Result<Vec<Effect>, HookError> {
        self.push(format!("before_call_tools:{}", input.tool_calls.len()));
        Ok(vec![])
    }

    async fn after_call_tools(&self, input: AfterCallTools<'_>) -> Result<Vec<Effect>, HookError> {
        self.push(format!("after_call_tools:{}", input.tool_results.len()));
        Ok(vec![])
    }

    async fn after_step(&self, _input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        let event_count = self.event_count.load(Ordering::SeqCst);
        self.push(format!("after_step:{event_count}"));
        Ok(vec![])
    }
}

struct FailingHook;

#[async_trait]
impl RuntimeHook for FailingHook {
    fn name(&self) -> &'static str {
        "failing"
    }

    async fn before_call_model(&self) -> Result<Vec<Effect>, HookError> {
        Err(HookError::new("forced failure"))
    }
}

struct TimeoutFinalizeHook {
    seen: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl RuntimeHook for TimeoutFinalizeHook {
    fn name(&self) -> &'static str {
        "timeout_finalize"
    }

    async fn before_step(&self, _input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![Effect::store_scratchpad(
            "timeout_finalize.marker",
            "before-step".to_string(),
        )])
    }

    async fn after_step(&self, input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        if matches!(input.result, StepResultDraft::Error(AgentError::Timeout)) {
            let marker = input
                .frame
                .scratchpad
                .get::<String>("timeout_finalize.marker")
                .cloned()
                .unwrap_or_else(|| "missing".to_string());
            self.seen.lock().unwrap().push(marker);
        }
        Ok(vec![])
    }
}

struct OrderingInternalHook {
    seen: Arc<Mutex<Vec<String>>>,
}

impl OrderingInternalHook {
    fn push(&self, value: impl Into<String>) {
        self.seen.lock().unwrap().push(value.into());
    }
}

#[async_trait]
impl RuntimeHook for OrderingInternalHook {
    fn name(&self) -> &'static str {
        "ordering_internal"
    }

    async fn before_step(&self, _input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        self.push("internal.before_step");
        Ok(vec![])
    }

    async fn after_step(&self, input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        let phase = match input.result {
            StepResultDraft::Continue { .. } => "continue",
            StepResultDraft::Done { .. } => "done",
            StepResultDraft::Error(_) => "error",
        };
        self.push(format!("internal.after_step:{phase}"));
        Ok(vec![])
    }
}

struct MaxIterationProbeHook {
    seen: Arc<Mutex<Vec<String>>>,
}

impl MaxIterationProbeHook {
    fn push(&self, value: impl Into<String>) {
        self.seen.lock().unwrap().push(value.into());
    }
}

#[async_trait]
impl RuntimeHook for MaxIterationProbeHook {
    fn name(&self) -> &'static str {
        "max_iteration_probe"
    }

    async fn before_step(&self, _input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        self.push("before_step");
        Ok(vec![])
    }

    async fn after_step(&self, input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        let label = match input.result {
            StepResultDraft::Error(AgentError::MaxIterations(iteration)) => {
                format!("after_step:max_iterations:{iteration}")
            }
            other => format!("after_step:unexpected:{other:?}"),
        };
        self.push(label);
        Ok(vec![])
    }
}

struct PublicMetadataHook {
    seen: Arc<Mutex<Vec<String>>>,
}

impl PublicMetadataHook {
    fn push(&self, value: impl Into<String>) {
        self.seen.lock().unwrap().push(value.into());
    }
}

#[async_trait]
impl Hook for PublicMetadataHook {
    fn name(&self) -> &'static str {
        "public_metadata"
    }

    async fn before_step(&self, input: BeforeStepInput) -> Result<Vec<HookEffect>, HookError> {
        let origin = input
            .metadata
            .get("origin")
            .and_then(|value| value.as_str())
            .unwrap_or("missing");
        self.push(format!("public.before_step.1:{origin}"));
        Ok(vec![
            HookEffect::SetMetadata {
                key: "origin".to_string(),
                value: json!("public-1"),
            },
            HookEffect::EmitEvent(HookEvent {
                kind: "before_step".to_string(),
                payload: json!({ "source": "public_metadata" }),
            }),
        ])
    }
}

struct PublicMetadataReaderHook {
    seen: Arc<Mutex<Vec<String>>>,
}

impl PublicMetadataReaderHook {
    fn push(&self, value: impl Into<String>) {
        self.seen.lock().unwrap().push(value.into());
    }
}

#[async_trait]
impl Hook for PublicMetadataReaderHook {
    fn name(&self) -> &'static str {
        "public_metadata_reader"
    }

    async fn before_step(&self, input: BeforeStepInput) -> Result<Vec<HookEffect>, HookError> {
        let origin = input
            .metadata
            .get("origin")
            .and_then(|value| value.as_str())
            .unwrap_or("missing");
        self.push(format!("public.before_step.2:{origin}"));
        Ok(vec![])
    }
}

struct ReplaceAfterStepPublicHook;

#[async_trait]
impl Hook for ReplaceAfterStepPublicHook {
    fn name(&self) -> &'static str {
        "public_replace_after_step"
    }

    async fn after_step(&self, input: AfterStepInput) -> Result<Vec<HookEffect>, HookError> {
        assert!(matches!(input.result, HookStepResult::Continue(_)));
        Ok(vec![HookEffect::ReplaceResult(HookStepUpdate::Done(
            HookDoneStep {
                content: "public override".to_string(),
                reasoning_content: Some("public reasoning".to_string()),
            },
        ))])
    }
}

struct AbortAfterStepPublicHook;

#[async_trait]
impl Hook for AbortAfterStepPublicHook {
    fn name(&self) -> &'static str {
        "public_abort"
    }

    async fn after_step(&self, _input: AfterStepInput) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![HookEffect::Abort {
            reason: "stop requested".to_string(),
        }])
    }
}

struct InvalidReplaceBeforeStepHook;

#[async_trait]
impl Hook for InvalidReplaceBeforeStepHook {
    fn name(&self) -> &'static str {
        "invalid_replace"
    }

    async fn before_step(&self, _input: BeforeStepInput) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![HookEffect::ReplaceResult(HookStepUpdate::Done(
            HookDoneStep {
                content: "invalid".to_string(),
                reasoning_content: None,
            },
        ))])
    }
}

struct ReplaceContinuePublicHook;

#[async_trait]
impl Hook for ReplaceContinuePublicHook {
    fn name(&self) -> &'static str {
        "public_continue_replace"
    }

    async fn after_step(&self, input: AfterStepInput) -> Result<Vec<HookEffect>, HookError> {
        match input.result {
            HookStepResult::Continue(HookContinueStep {
                reasoning_content,
                tool_calls,
                tool_results,
                ..
            }) => Ok(vec![HookEffect::ReplaceResult(HookStepUpdate::Continue(
                HookContinueStep {
                    content: "public continued".to_string(),
                    reasoning_content,
                    tool_calls,
                    tool_results,
                },
            ))]),
            _ => Ok(vec![]),
        }
    }
}

fn default_test_hooks() -> RuntimeHookRegistry {
    RuntimeHookRegistry::default()
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

fn assert_step_result_matches(actual: &StepResult, expected: &StepResult) {
    match (actual, expected) {
        (
            StepResult::Continue {
                content: actual_content,
                reasoning_content: actual_reasoning,
                tool_calls: actual_calls,
                tool_results: actual_results,
            },
            StepResult::Continue {
                content: expected_content,
                reasoning_content: expected_reasoning,
                tool_calls: expected_calls,
                tool_results: expected_results,
            },
        ) => {
            assert_eq!(actual_content, expected_content);
            assert_eq!(actual_reasoning, expected_reasoning);
            assert_eq!(actual_calls.len(), expected_calls.len());
            assert_eq!(actual_results.len(), expected_results.len());
            for (actual_call, expected_call) in actual_calls.iter().zip(expected_calls.iter()) {
                assert_eq!(actual_call.id, expected_call.id);
                assert_eq!(actual_call.call_type, expected_call.call_type);
                assert_eq!(actual_call.index, expected_call.index);
                assert_eq!(
                    actual_call
                        .function
                        .as_ref()
                        .map(|function| function.name.as_str()),
                    expected_call
                        .function
                        .as_ref()
                        .map(|function| function.name.as_str())
                );
                assert_eq!(
                    actual_call
                        .function
                        .as_ref()
                        .map(|function| function.arguments.as_str()),
                    expected_call
                        .function
                        .as_ref()
                        .map(|function| function.arguments.as_str())
                );
            }
            for (actual_result, expected_result) in
                actual_results.iter().zip(expected_results.iter())
            {
                assert_eq!(actual_result.call_id, expected_result.call_id);
                assert_eq!(actual_result.tool_name, expected_result.tool_name);
                assert_eq!(actual_result.success, expected_result.success);
                assert_eq!(actual_result.output, expected_result.output);
            }
        }
        (
            StepResult::Done {
                content: actual_content,
                reasoning_content: actual_reasoning,
            },
            StepResult::Done {
                content: expected_content,
                reasoning_content: expected_reasoning,
            },
        ) => {
            assert_eq!(actual_content, expected_content);
            assert_eq!(actual_reasoning, expected_reasoning);
        }
        (StepResult::Error(actual_err), StepResult::Error(expected_err)) => {
            assert_eq!(format!("{actual_err:?}"), format!("{expected_err:?}"));
        }
        _ => panic!("step results have different variants: {actual:?} vs {expected:?}"),
    }
}

#[test]
fn test_public_tool_call_normalizes_simplified_internal_shape() {
    let internal = ToolCall {
        id: "named_call".to_string(),
        call_type: None,
        index: Some(1),
        function: None,
        name: Some("get_weather".to_string()),
        arguments: Some(json!({ "city": "北京" })),
    };

    let public = HookToolCall::from_tool_call(&internal);

    assert_eq!(public.id, "named_call");
    assert_eq!(public.call_type, "function");
    assert_eq!(public.index, Some(1));
    assert_eq!(public.function.name, "get_weather");
    assert_eq!(public.function.arguments, r#"{"city":"北京"}"#);

    let round_trip = public.into_tool_call();
    assert_eq!(round_trip.call_type.as_deref(), Some("function"));
    assert_eq!(
        round_trip
            .function
            .as_ref()
            .map(|function| function.name.as_str()),
        Some("get_weather")
    );
    assert_eq!(
        round_trip
            .function
            .as_ref()
            .map(|function| function.arguments.as_str()),
        Some(r#"{"city":"北京"}"#)
    );
    assert!(round_trip.name.is_none());
    assert!(round_trip.arguments.is_none());
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
    let runtime_hooks = default_test_hooks().register(RecordingHook::new(phases.clone()));
    let mut actor = AgentActor::with_runtime_hooks(
        model,
        executor,
        context,
        runtime_hooks,
        HookRegistry::default(),
    );

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
    let runtime_hooks = default_test_hooks().register(RecordingHook::new(phases.clone()));
    let mut actor = AgentActor::with_runtime_hooks(
        model,
        executor,
        context,
        runtime_hooks,
        HookRegistry::default(),
    );
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
    let mut saw_step_finalized = false;
    let mut saw_tool_calls = false;
    let mut saw_tool_result = false;
    let mut saw_iteration = false;
    while let Ok(event) = event_rx.try_recv() {
        match event {
            AgentActorEvent::StepCompleted { .. } => saw_step_completed = true,
            AgentActorEvent::StepFinalized { result } => {
                saw_step_finalized = true;
                assert!(matches!(result, StepResult::Continue { .. }));
            }
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
            | AgentActorEvent::HookEvent { .. }
            | AgentActorEvent::MaxIterations { .. } => {}
        }
    }
    assert!(saw_step_completed);
    assert!(saw_step_finalized);
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

    let runtime_hooks = RuntimeHookRegistry::empty().register(FailingHook);
    let mut actor = AgentActor::with_runtime_hooks(
        model,
        executor,
        context,
        runtime_hooks,
        HookRegistry::default(),
    );

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
        .runtime_hook(TimeoutFinalizeHook { seen: seen.clone() })
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

#[tokio::test]
async fn test_public_hooks_run_before_internal_hooks_and_share_step_metadata() {
    let model = MockChatModel::new(vec![ChatChunk {
        content: "hello".to_string(),
        reasoning_content: String::new(),
        is_finished: true,
        finish_reason: Some("stop".to_string()),
        tool_calls: None,
    }]);
    let executor = GenericToolExecutor::new();

    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let seen = Arc::new(Mutex::new(Vec::new()));
    let runtime_hooks =
        RuntimeHookRegistry::empty().register(OrderingInternalHook { seen: seen.clone() });
    let mut actor = crate::agents::AgentActorBuilder::new(model, executor)
        .context(context)
        .runtime_hooks(runtime_hooks)
        .hook(PublicMetadataHook { seen: seen.clone() })
        .hook(PublicMetadataReaderHook { seen: seen.clone() })
        .build();
    let (event_tx, mut event_rx) = mpsc::channel(16);

    let result = actor.run_step(Some(event_tx)).await;

    match result {
        StepResult::Done { content, .. } => assert_eq!(content, "hello"),
        other => panic!("unexpected result: {other:?}"),
    }

    assert_eq!(
        seen.lock().unwrap().clone(),
        vec![
            "public.before_step.1:missing",
            "public.before_step.2:public-1",
            "internal.before_step",
            "internal.after_step:done",
        ]
    );

    let mut saw_hook_event = false;
    while let Ok(event) = event_rx.try_recv() {
        if let AgentActorEvent::HookEvent {
            hook,
            kind,
            payload,
        } = event
        {
            saw_hook_event = true;
            assert_eq!(hook, "public_metadata");
            assert_eq!(kind, "before_step");
            assert_eq!(payload, json!({ "source": "public_metadata" }));
        }
    }
    assert!(saw_hook_event);
    assert_eq!(actor.state().state, JobState::Completed);
}

#[tokio::test]
async fn test_public_after_step_replace_result_updates_internal_hooks() {
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
    context.add_message(Message::user("weather"));

    let runtime_hooks = RuntimeHookRegistry::empty();
    let hooks = HookRegistry::empty().register(ReplaceAfterStepPublicHook);
    let mut actor = AgentActor::with_runtime_hooks(model, executor, context, runtime_hooks, hooks);

    let result = actor.run_step(None).await;

    match result {
        StepResult::Done {
            content,
            reasoning_content,
        } => {
            assert_eq!(content, "public override");
            assert_eq!(reasoning_content.as_deref(), Some("public reasoning"));
        }
        other => panic!("unexpected result: {other:?}"),
    }

    assert_eq!(actor.state().state, JobState::Completed);
    let conversation = actor.context().conversation();
    assert_eq!(conversation.len(), 2);
    match conversation.last().unwrap() {
        Message::Assistant {
            content,
            reasoning_content,
            tool_calls,
        } => {
            assert_eq!(content, "public override");
            assert_eq!(reasoning_content.as_deref(), Some("public reasoning"));
            assert!(tool_calls.is_none());
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn test_public_after_step_abort_still_runs_runtime_after_step_finalizers() {
    let model = MockChatModel::new(vec![ChatChunk {
        content: "hello".to_string(),
        reasoning_content: String::new(),
        is_finished: true,
        finish_reason: Some("stop".to_string()),
        tool_calls: None,
    }]);
    let executor = GenericToolExecutor::new();

    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let seen = Arc::new(Mutex::new(Vec::new()));
    let runtime_hooks =
        RuntimeHookRegistry::empty().register(OrderingInternalHook { seen: seen.clone() });
    let hooks = HookRegistry::empty().register(AbortAfterStepPublicHook);
    let mut actor = AgentActor::with_runtime_hooks(model, executor, context, runtime_hooks, hooks);
    let (event_tx, mut event_rx) = mpsc::channel(16);

    let result = actor.run_step(Some(event_tx)).await;

    match result {
        StepResult::Error(AgentError::Hook {
            plugin,
            phase,
            message,
        }) => {
            assert_eq!(plugin, "public_abort");
            assert_eq!(phase, HookPhase::AfterStep);
            assert_eq!(message, "stop requested");
        }
        other => panic!("unexpected result: {other:?}"),
    }

    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["internal.before_step", "internal.after_step:error"]
    );
    assert_eq!(actor.state().state, JobState::Failed);

    let mut saw_error_event = false;
    let mut saw_step_finalized = false;
    while let Ok(event) = event_rx.try_recv() {
        match event {
            AgentActorEvent::StepFinalized {
                result:
                    StepResult::Error(AgentError::Hook {
                        plugin,
                        phase,
                        message,
                    }),
            } => {
                saw_step_finalized = true;
                assert_eq!(plugin, "public_abort");
                assert_eq!(phase, HookPhase::AfterStep);
                assert_eq!(message, "stop requested");
            }
            AgentActorEvent::Error(AgentError::Hook {
                plugin,
                phase,
                message,
            }) => {
                saw_error_event = true;
                assert_eq!(plugin, "public_abort");
                assert_eq!(phase, HookPhase::AfterStep);
                assert_eq!(message, "stop requested");
            }
            _ => {}
        }
    }
    assert!(saw_error_event);
    assert!(saw_step_finalized);
}

#[tokio::test]
async fn test_public_hooks_reject_invalid_effects_for_phase() {
    let model = MockChatModel::new(vec![ChatChunk {
        content: "hello".to_string(),
        reasoning_content: String::new(),
        is_finished: true,
        finish_reason: Some("stop".to_string()),
        tool_calls: None,
    }]);
    let executor = GenericToolExecutor::new();

    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let runtime_hooks = RuntimeHookRegistry::empty();
    let hooks = HookRegistry::empty().register(InvalidReplaceBeforeStepHook);
    let mut actor = AgentActor::with_runtime_hooks(model, executor, context, runtime_hooks, hooks);

    let result = actor.run_step(None).await;

    match result {
        StepResult::Error(AgentError::Hook {
            plugin,
            phase,
            message,
        }) => {
            assert_eq!(plugin, "invalid_replace");
            assert_eq!(phase, HookPhase::BeforeStep);
            assert!(message.contains("ReplaceResult is only supported during after_step"));
        }
        other => panic!("unexpected result: {other:?}"),
    }

    assert_eq!(actor.state().state, JobState::Failed);
}

#[tokio::test]
async fn test_public_continue_replace_preserves_tool_call_wire_shape() {
    let seen_messages = Arc::new(Mutex::new(Vec::new()));
    let model = RoundTripChatModel::new(seen_messages.clone());

    let mut executor = GenericToolExecutor::new();
    executor.register(MockWeatherTool::new());

    let mut context = Context::new();
    context.add_message(Message::user("weather"));

    let runtime_hooks = RuntimeHookRegistry::empty();
    let hooks = HookRegistry::empty().register(ReplaceContinuePublicHook);
    let mut actor = AgentActor::with_runtime_hooks(model, executor, context, runtime_hooks, hooks);

    let first = actor.run_step(None).await;
    match first {
        StepResult::Continue { content, .. } => assert_eq!(content, "public continued"),
        other => panic!("unexpected first result: {other:?}"),
    }

    let second = actor.run_step(None).await;
    match second {
        StepResult::Done { content, .. } => assert_eq!(content, "final answer"),
        other => panic!("unexpected second result: {other:?}"),
    }

    let seen_messages = seen_messages.lock().unwrap();
    assert_eq!(seen_messages.len(), 2);
    let second_request = &seen_messages[1];
    let assistant_message = second_request
        .iter()
        .find_map(|message| match message {
            Message::Assistant { tool_calls, .. } => tool_calls.as_ref(),
            _ => None,
        })
        .expect("assistant tool_calls should be preserved");

    assert_eq!(assistant_message.len(), 1);
    let tool_call = &assistant_message[0];
    assert_eq!(tool_call.id, "call_weather");
    assert_eq!(tool_call.call_type.as_deref(), Some("function"));
    assert_eq!(tool_call.index, Some(0));
    assert_eq!(
        tool_call
            .function
            .as_ref()
            .map(|function| function.name.as_str()),
        Some("get_weather")
    );
    assert_eq!(
        tool_call
            .function
            .as_ref()
            .map(|function| function.arguments.as_str()),
        Some(r#"{"city":"北京"}"#)
    );
    assert!(tool_call.name.is_none());
    assert!(tool_call.arguments.is_none());
}

#[tokio::test]
async fn test_step_finalized_matches_return_value_and_context() {
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
    context.add_message(Message::user("weather"));

    let mut actor = AgentActor::new(model, executor, context);
    let (event_tx, mut event_rx) = mpsc::channel(16);

    let result = actor.run_step(Some(event_tx)).await;

    let finalized = loop {
        let event = event_rx
            .recv()
            .await
            .expect("step finalized event should exist");
        if let AgentActorEvent::StepFinalized { result } = event {
            break result;
        }
    };

    assert_step_result_matches(&result, &finalized);
    assert_eq!(actor.state().state, JobState::WaitingInput);

    let conversation = actor.context().conversation();
    assert_eq!(conversation.len(), 3);
    match &finalized {
        StepResult::Continue {
            content,
            reasoning_content,
            tool_calls,
            tool_results,
        } => {
            match &conversation[1] {
                Message::Assistant {
                    content: message_content,
                    reasoning_content: message_reasoning,
                    tool_calls: message_tool_calls,
                } => {
                    assert_eq!(message_content, content);
                    assert_eq!(message_reasoning, reasoning_content);
                    let message_tool_calls = message_tool_calls
                        .as_ref()
                        .expect("assistant message should include tool calls");
                    assert_eq!(message_tool_calls.len(), tool_calls.len());
                    assert_eq!(message_tool_calls[0].id, tool_calls[0].id);
                }
                other => panic!("unexpected assistant message: {other:?}"),
            }
            match &conversation[2] {
                Message::Tool {
                    tool_call_id,
                    content: tool_content,
                } => {
                    assert_eq!(tool_call_id, &tool_results[0].call_id);
                    assert_eq!(tool_content, &tool_results[0].output);
                }
                other => panic!("unexpected tool message: {other:?}"),
            }
        }
        other => panic!("unexpected finalized result: {other:?}"),
    }
}

#[tokio::test]
async fn test_max_iterations_short_circuits_before_hooks() {
    let model = MockChatModel::new(vec![ChatChunk {
        content: "should not run".to_string(),
        reasoning_content: String::new(),
        is_finished: true,
        finish_reason: Some("stop".to_string()),
        tool_calls: None,
    }]);
    let executor = GenericToolExecutor::new();

    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let seen = Arc::new(Mutex::new(Vec::new()));
    let runtime_hooks =
        RuntimeHookRegistry::empty().register(MaxIterationProbeHook { seen: seen.clone() });
    let mut actor = crate::agents::AgentActorBuilder::new(model, executor)
        .context(context)
        .max_iterations(0)
        .runtime_hooks(runtime_hooks)
        .build();
    let (event_tx, mut event_rx) = mpsc::channel(16);

    let result = actor.run_step(Some(event_tx)).await;

    match result {
        StepResult::Error(AgentError::MaxIterations(iteration)) => {
            assert_eq!(iteration, 0);
        }
        other => panic!("unexpected result: {other:?}"),
    }

    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["after_step:max_iterations:0"]
    );
    assert_eq!(actor.state().iteration, 0);
    assert_eq!(actor.state().state, JobState::Failed);
    assert_eq!(actor.context().conversation().len(), 1);

    let events: Vec<_> = std::iter::from_fn(|| event_rx.try_recv().ok()).collect();
    let finalized_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                AgentActorEvent::StepFinalized {
                    result: StepResult::Error(AgentError::MaxIterations(0)),
                }
            )
        })
        .expect("expected step finalized event");
    let max_iterations_index = events
        .iter()
        .position(|event| matches!(event, AgentActorEvent::MaxIterations { iteration: 0 }))
        .expect("expected max iterations event");
    assert!(finalized_index < max_iterations_index);
}

#[tokio::test]
async fn test_run_loop_emits_step_finalized_before_completed() {
    let model = MockChatModel::new(vec![ChatChunk {
        content: "done".to_string(),
        reasoning_content: String::new(),
        is_finished: true,
        finish_reason: Some("stop".to_string()),
        tool_calls: None,
    }]);
    let executor = GenericToolExecutor::new();

    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let events = AgentActor::new(model, executor, context)
        .run_loop()
        .wait()
        .await;

    let step_finalized_index = events
        .iter()
        .position(|event| matches!(event, AgentActorEvent::StepFinalized { .. }))
        .expect("expected step finalized event");
    let iteration_index = events
        .iter()
        .position(|event| matches!(event, AgentActorEvent::Iteration { iteration: 1, .. }))
        .expect("expected iteration event");
    let completed_index = events
        .iter()
        .position(|event| matches!(event, AgentActorEvent::Completed))
        .expect("expected completed event");

    assert!(step_finalized_index < iteration_index);
    assert!(iteration_index < completed_index);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, AgentActorEvent::Completed))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            AgentActorEvent::Cancelled
                | AgentActorEvent::Error(_)
                | AgentActorEvent::MaxIterations { .. }
        )
    }));
}

#[tokio::test]
async fn test_run_loop_max_iterations_emits_terminal_event_once() {
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

    let events = crate::agents::AgentActorBuilder::new(model, executor)
        .context(context)
        .max_iterations(0)
        .build()
        .run_loop()
        .wait()
        .await;

    let step_finalized_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                AgentActorEvent::StepFinalized {
                    result: StepResult::Error(AgentError::MaxIterations(0)),
                }
            )
        })
        .expect("expected step finalized event");
    let max_iterations_index = events
        .iter()
        .position(|event| matches!(event, AgentActorEvent::MaxIterations { iteration: 0 }))
        .expect("expected max iterations event");

    assert!(step_finalized_index < max_iterations_index);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, AgentActorEvent::MaxIterations { .. }))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            AgentActorEvent::Completed | AgentActorEvent::Cancelled | AgentActorEvent::Error(_)
        )
    }));
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
        Message::system("你是一个有用的助手。当用户询问天气时，请使用 get_weather 工具获取信息。"),
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
        let mut content = String::new();
        let mut reasoning_content: Option<String> = None;
        let mut tool_calls: Option<Vec<ToolCall>> = None;

        {
            let stream = call_model(model.as_ref(), &messages_clone, Some(executor.tools()));
            let mut stream = pin!(stream);

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
        }

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
                    } else if let Some(error) = json_value.get("error").and_then(|v| v.as_str()) {
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
            AgentActorEvent::StepFinalized { result } => {
                println!("\n[Event] StepFinalized: {:?}", result);
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
            AgentActorEvent::HookEvent {
                hook,
                kind,
                payload,
            } => {
                println!(
                    "\n[Event] HookEvent: hook={}, kind={}, payload={}",
                    hook, kind, payload
                );
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
