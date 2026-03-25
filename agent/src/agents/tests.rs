use crate::agents::tools::playwright_cli::PlaywrightCliTool;
use crate::agents::{AgentActor, AgentActorEvent, Context, GenericToolExecutor, ToolDef};

use crate::core::{Message, Usage};
use crate::models::{ChatCapability, ChatChunk, ChatError, ChatModel};
use crate::providers::deepseek_provider_from_env;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use serde_json::Value;
use std::sync::Arc;

struct MockChatModel {
    chunks: Vec<ChatChunk>,
}

#[async_trait]
impl ChatCapability for MockChatModel {
    async fn chat(
        &self,
        _msgs: Vec<Message>,
        _tools: Option<Vec<ToolDef>>,
    ) -> Result<Message, ChatError> {
        panic!("chat should not be called in this test");
    }

    async fn chat_stream(
        &self,
        _msgs: Vec<Message>,
        _tools: Option<Vec<ToolDef>>,
    ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
        Ok(Box::pin(stream::iter(self.chunks.clone())))
    }
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

#[tokio::test]
async fn test_agent_actor_accumulates_usage_from_stream_response() {
    let model = MockChatModel {
        chunks: vec![ChatChunk {
            content: "done".to_string(),
            reasoning_content: String::new(),
            is_finished: true,
            finish_reason: Some("stop".to_string()),
            tool_calls: None,
            usage: Some(Usage {
                prompt_tokens: 13,
                completion_tokens: 8,
                total_tokens: 21,
            }),
        }],
    };

    let executor = GenericToolExecutor::new();
    let mut context = Context::new();
    context.add_message(Message::user("hello"));

    let mut actor = AgentActor::new(model, executor, context);
    let result = actor.run_step(None).await;

    assert!(matches!(result, crate::agents::StepResult::Done { .. }));
    assert_eq!(actor.state().token_statistics.prompt_tokens, 13);
    assert_eq!(actor.state().token_statistics.completion_tokens, 8);
    assert_eq!(actor.state().token_statistics.total_tokens, 21);
}
