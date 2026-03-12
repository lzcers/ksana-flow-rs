//! Actor 模型 - 可暂停、可恢复的执行单元
//!
//! Actor 是一个独立执行单元，具有以下特性：
//! - 可插拔的 Chat 模型和 ToolExecutor
//! - 支持暂停、继续、取消操作
//! - 状态可持久化和恢复
//! - 流式事件输出

use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agents::{AgentState, Context, JobState, ToolCall, ToolExecutor};
use crate::core::Message;
use crate::models::{ChatCapability, ChatChunk};

// ============================================================================
// Actor 事件
// ============================================================================

/// Actor 产生的事件
#[derive(Debug, Clone)]
pub enum ActorEvent {
    /// 流式输出片段
    Chunk(String),
    /// 模型请求工具调用
    ToolCalls(Vec<ToolCall>),
    /// 单个工具执行完成
    ToolResult {
        call_id: String,
        success: bool,
        output: String,
    },
    /// 一次迭代完成
    Iteration {
        iteration: usize,
        message_count: usize,
    },
    /// Agent 执行完成
    Completed,
    /// 发生错误
    Error(String),
}

// ============================================================================
// Actor 命令
// ============================================================================

/// 发送给 Actor 的控制命令
#[derive(Debug)]
pub enum ActorCommand {
    /// 暂停执行
    Pause,
    /// 继续执行（恢复）
    Continue,
    /// 取消执行
    Cancel,
}

// ============================================================================
// Actor Handle
// ============================================================================

/// Actor 的外部控制句柄
pub struct ActorHandle {
    /// 命令发送器
    pub cmd_tx: mpsc::Sender<ActorCommand>,
    /// 事件接收器
    pub event_rx: mpsc::Receiver<ActorEvent>,
}

impl ActorHandle {
    /// 暂停 Actor
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(ActorCommand::Pause).await;
    }

    /// 继续/恢复 Actor
    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(ActorCommand::Continue).await;
    }

    /// 取消 Actor
    pub async fn cancel(&self) {
        let _ = self.cmd_tx.send(ActorCommand::Cancel).await;
    }
}

// ============================================================================
// Agent Actor
// ============================================================================

/// Agent Actor - 可插拔的 LLM Agent 执行器
///
/// 泛型参数：
/// - `C`: Chat 模型能力
/// - `E`: 工具执行器
pub struct AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send + Sync,
{
    /// Agent 状态
    state: AgentState,
    /// Chat 模型
    chat: Arc<C>,
    /// 工具执行器
    executor: Arc<E>,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + Sync + 'static,
{
    /// 创建新的 Agent Actor
    pub fn new(chat: Arc<C>, executor: Arc<E>, context: Context) -> Self {
        Self {
            state: AgentState {
                job_id: Uuid::new_v4(),
                user_id: "default".to_string(),
                conversation_id: None,
                title: String::new(),
                description: String::new(),
                category: None,
                state: JobState::Pending,
                budget: None,
                actual_cost: rust_decimal::Decimal::ZERO,
                context,
                iteration: 0,
                max_iterations: 10,
            },
            chat,
            executor,
        }
    }

    /// 设置最大迭代次数
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.state.max_iterations = max;
        self
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.state.user_id = user_id.into();
        self
    }

    /// 获取状态
    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// 获取可变状态
    pub fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }

    /// 启动 Actor，返回控制句柄
    pub fn spawn(mut self) -> ActorHandle {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<ActorCommand>(16);
        let (event_tx, event_rx) = mpsc::channel::<ActorEvent>(64);

        tokio::spawn(async move {
            let mut paused = false;
            let mut cancelled = false;

            // 更新状态为 Running
            self.state.state = JobState::Running;

            loop {
                // 检查命令
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        ActorCommand::Pause => {
                            paused = true;
                            self.state.state = JobState::Paused;
                        }
                        ActorCommand::Continue => {
                            paused = false;
                            if self.state.state == JobState::Paused {
                                self.state.state = JobState::Running;
                            }
                        }
                        ActorCommand::Cancel => {
                            cancelled = true;
                            self.state.state = JobState::Cancelled;
                        }
                    }
                }

                if cancelled {
                    let _ = event_tx.send(ActorEvent::Error("Cancelled".to_string())).await;
                    break;
                }

                if paused {
                    // 等待继续命令
                    if let Some(ActorCommand::Continue) = cmd_rx.recv().await {
                        paused = false;
                        self.state.state = JobState::Running;
                    }
                    continue;
                }

                // 执行一步迭代
                let result = self.step(&event_tx).await;

                match result {
                    Ok(true) => {
                        // 继续下一次迭代
                        continue;
                    }
                    Ok(false) => {
                        // 执行完成
                        let _ = event_tx.send(ActorEvent::Completed).await;
                        break;
                    }
                    Err(e) => {
                        let _ = event_tx.send(ActorEvent::Error(e)).await;
                        break;
                    }
                }
            }
        });

        ActorHandle { cmd_tx, event_rx }
    }

    /// 执行一次迭代
    /// 返回 Ok(true) 表示应继续，Ok(false) 表示完成
    async fn step(
        &mut self,
        event_tx: &mpsc::Sender<ActorEvent>,
    ) -> Result<bool, String> {
        self.state.iteration += 1;

        if self.state.iteration > self.state.max_iterations {
            return Ok(false);
        }

        // 获取消息列表
        let messages = self.state.context.to_messages();
        let tools = self.executor.tools().clone();

        // 流式调用模型
        let stream = self
            .chat
            .chat_stream(messages, Some(tools))
            .await
            .map_err(|e| e.to_string())?;

        // 收集流式响应
        let (content, tool_calls) = collect_stream_with_events(stream, event_tx).await;

        // 构建助手消息
        let assistant_msg = Message::Assistant {
            content,
            tool_calls: tool_calls.clone(),
        };

        // 添加到上下文
        self.state.context.add_message(assistant_msg);

        // 检查是否有工具调用
        if let Some(calls) = tool_calls {
            // 发送工具调用事件
            let _ = event_tx.send(ActorEvent::ToolCalls(calls.clone())).await;

            // 直接执行工具调用（避免使用 call_tools 的 &dyn 问题）
            for call in calls {
                let tool_call_id = call.id.clone();
                let result = self.executor.execute(call).await;

                let (success, output) = match result {
                    Ok(r) => (r.success, serde_json::to_string(&r.output).unwrap_or_default()),
                    Err(e) => (false, e.to_string()),
                };

                // 添加工具消息
                self.state.context.add_message(Message::Tool {
                    tool_call_id: tool_call_id.clone(),
                    content: output.clone(),
                });

                // 发送工具结果事件
                let _ = event_tx
                    .send(ActorEvent::ToolResult {
                        call_id: tool_call_id,
                        success,
                        output,
                    })
                    .await;
            }

            // 发送迭代事件
            let _ = event_tx
                .send(ActorEvent::Iteration {
                    iteration: self.state.iteration,
                    message_count: self.state.context.conversation().len(),
                })
                .await;

            Ok(true) // 继续迭代
        } else {
            // 没有工具调用，完成
            Ok(false)
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 收集流式响应并发送事件
async fn collect_stream_with_events(
    mut stream: futures::stream::BoxStream<'static, ChatChunk>,
    event_tx: &mpsc::Sender<ActorEvent>,
) -> (String, Option<Vec<ToolCall>>) {
    let mut content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    while let Some(chunk) = stream.next().await {
        // 发送 chunk 事件
        if !chunk.content.is_empty() {
            let _ = event_tx.send(ActorEvent::Chunk(chunk.content.clone())).await;
            content.push_str(&chunk.content);
        }

        // 合并工具调用
        if let Some(inc_tool_calls) = chunk.tool_calls {
            merge_tool_calls(&mut tool_calls, inc_tool_calls);
        }
    }

    let final_tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    (content, final_tool_calls)
}

/// 合并增量 ToolCall
fn merge_tool_calls(accumulated: &mut Vec<ToolCall>, incremental: Vec<ToolCall>) {
    for inc in incremental {
        let existing = accumulated.iter_mut().find(|tc| {
            if let (Some(idx1), Some(idx2)) = (tc.index, inc.index) {
                idx1 == idx2
            } else if !tc.id.is_empty() && tc.id == inc.id {
                true
            } else {
                false
            }
        });

        if let Some(existing) = existing {
            if !inc.id.is_empty() {
                existing.id = inc.id;
            }
            if inc.call_type.is_some() {
                existing.call_type = inc.call_type;
            }
            if inc.index.is_some() {
                existing.index = inc.index;
            }
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
            accumulated.push(inc);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{ToolDef, ToolExecutorError, ToolResult};
    use crate::models::ChatError;
    use async_trait::async_trait;
    use futures::stream::BoxStream;
    use serde_json::json;

    // Mock Chat Model
    struct MockChat;

    #[async_trait]
    impl ChatCapability for MockChat {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, ChatError> {
            Ok(Message::assistant("Mock response"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
            let chunks = vec![
                ChatChunk {
                    content: "Hello".to_string(),
                    is_finished: false,
                    finish_reason: None,
                    tool_calls: None,
                },
                ChatChunk {
                    content: "!".to_string(),
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                    tool_calls: None,
                },
            ];
            Ok(futures::stream::iter(chunks).boxed())
        }
    }

    // Mock Tool Executor
    struct MockExecutor {
        tools: Vec<ToolDef>,
    }

    #[async_trait]
    impl ToolExecutor for MockExecutor {
        async fn execute(&self, _call: ToolCall) -> Result<ToolResult, ToolExecutorError> {
            Ok(ToolResult {
                id: "test".to_string(),
                success: true,
                output: json!({"result": "ok"}),
            })
        }

        fn tools(&self) -> &Vec<ToolDef> {
            &self.tools
        }
    }

    #[tokio::test]
    async fn test_actor_spawn_and_collect() {
        let chat = Arc::new(MockChat);
        let executor = Arc::new(MockExecutor { tools: vec![] });
        let context = Context::new();

        let actor = AgentActor::new(chat, executor, context).with_max_iterations(1);
        let mut handle = actor.spawn();

        // 收集事件
        let mut events = Vec::new();
        while let Some(event) = handle.event_rx.recv().await {
            events.push(event);
            if matches!(events.last(), Some(ActorEvent::Completed)) {
                break;
            }
        }

        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, ActorEvent::Chunk(_))));
        assert!(events.iter().any(|e| matches!(e, ActorEvent::Completed)));
    }

    #[tokio::test]
    async fn test_actor_pause_and_resume() {
        let chat = Arc::new(MockChat);
        let executor = Arc::new(MockExecutor { tools: vec![] });
        let context = Context::new();

        let actor = AgentActor::new(chat, executor, context).with_max_iterations(2);
        let mut handle = actor.spawn();

        // 暂停
        handle.pause().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 恢复
        handle.resume().await;

        // 收集事件
        let mut completed = false;
        while let Some(event) = handle.event_rx.recv().await {
            if matches!(event, ActorEvent::Completed) {
                completed = true;
                break;
            }
        }

        assert!(completed);
    }

    #[tokio::test]
    async fn test_actor_with_tool_calls() {
        // Mock Chat 返回工具调用
        struct MockChatWithTools;

        #[async_trait]
        impl ChatCapability for MockChatWithTools {
            async fn chat(
                &self,
                _msgs: Vec<Message>,
                _tools: Option<Vec<ToolDef>>,
            ) -> Result<Message, ChatError> {
                Ok(Message::assistant("Done"))
            }

            async fn chat_stream(
                &self,
                _msgs: Vec<Message>,
                _tools: Option<Vec<ToolDef>>,
            ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
                // 第一次调用返回工具调用
                let chunks = vec![
                    ChatChunk {
                        content: String::new(),
                        is_finished: false,
                        finish_reason: None,
                        tool_calls: Some(vec![ToolCall {
                            id: "call_123".to_string(),
                            call_type: Some("function".to_string()),
                            index: Some(0),
                            function: Some(crate::agents::ToolCallFunction {
                                name: "test_tool".to_string(),
                                arguments: r#"{"arg":"value"}"#.to_string(),
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
                ];
                Ok(futures::stream::iter(chunks).boxed())
            }
        }

        // Mock Executor
        let tools = vec![ToolDef {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({"type": "object"}),
        }];
        let executor = Arc::new(MockExecutor { tools });
        let chat = Arc::new(MockChatWithTools);

        let context = Context::new();
        let actor = AgentActor::new(chat, executor, context).with_max_iterations(2);
        let mut handle = actor.spawn();

        // 收集事件
        let mut events = Vec::new();
        while let Some(event) = handle.event_rx.recv().await {
            events.push(event.clone());
            if matches!(event, ActorEvent::Completed) {
                break;
            }
        }

        // 验证有工具调用事件
        assert!(events.iter().any(|e| matches!(e, ActorEvent::ToolCalls(_))));
        assert!(events.iter().any(|e| matches!(e, ActorEvent::ToolResult { .. })));
    }
}