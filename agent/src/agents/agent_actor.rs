//! Actor 控制器 - 命令处理、状态管理、事件发送、迭代控制
//!
//! Actor 是一个独立执行单元，具有以下特性：
//! - 可插拔的 Chat 模型和 ToolExecutor
//! - 支持暂停、继续、取消操作
//! - 状态可持久化和恢复
//! - 流式事件输出
//!
//! 职责分离：
//! - 执行逻辑由纯函数 `call_model`、`call_tools` 处理
//! - 本文件负责控制面：命令处理、状态转换、事件发送、迭代控制

use std::pin::pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::agent_utils::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::{AgentState, Context, ToolCall, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

// ============================================================================
// 错误类型
// ============================================================================

/// Agent 执行错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    /// 模型调用错误
    #[error("Model error: {0}")]
    Model(String),
    /// 工具执行错误
    #[error("Tool execution error: {0}")]
    Tool(String),
    /// 用户取消
    #[error("Cancelled by user")]
    Cancelled,
    /// 操作超时
    #[error("Operation timed out")]
    Timeout,
    /// 达到最大迭代次数
    #[error("Max iterations ({0}) reached")]
    MaxIterations(usize),
}

// ============================================================================
// 执行指标
// ============================================================================

/// 执行统计指标
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    /// 总执行时长
    pub total_duration: Duration,
    /// 工具调用次数
    pub tool_calls_count: usize,
    /// 迭代次数
    pub iterations: usize,
}

// ============================================================================
// 事件类型
// ============================================================================

/// Actor 产生的事件
#[derive(Debug, Clone)]
pub enum AgentActorEvent {
    /// 流式输出片段
    ContentChunk(String),
    /// 推理内容片段（DeepSeek 推理模式）
    ReasoningChunk(String),
    /// LLM 响应完成，包含完整的 content 和 tool_calls
    StepCompleted {
        /// 完整的响应内容
        content: String,
        /// 完整的推理内容（如果有）
        reasoning_content: Option<String>,
        /// 工具调用列表（如果有）
        tool_calls: Option<Vec<ToolCall>>,
    },
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
    /// 达到最大迭代次数
    MaxIterations { iteration: usize },
    /// Agent 执行完成
    Completed,
    /// 用户取消执行
    Cancelled,
    /// 发生错误
    Error(AgentError),
}

// ============================================================================
// 命令类型
// ============================================================================

/// 发送给 Actor 的控制命令
#[derive(Debug)]
pub enum AgentActorCommand {
    /// 暂停执行
    Pause,
    /// 继续执行（恢复）
    Continue,
    /// 取消执行
    Cancel,
}

// ============================================================================
// Actor 句柄
// ============================================================================

/// Actor 的外部控制句柄
pub struct AgentActorHandle {
    /// 命令发送器
    pub cmd_tx: mpsc::Sender<AgentActorCommand>,
    /// 事件接收器
    pub event_rx: mpsc::Receiver<AgentActorEvent>,
}

impl AgentActorHandle {
    /// 暂停 Actor
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Pause).await;
    }

    /// 继续/恢复 Actor
    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Continue).await;
    }

    /// 取消 Actor
    pub async fn cancel(&self) {
        let _ = self.cmd_tx.send(AgentActorCommand::Cancel).await;
    }

    /// 检查 actor 是否已完成（事件 channel 已关闭）
    pub fn is_finished(&self) -> bool {
        self.event_rx.is_closed()
    }

    /// 等待 Actor 完成，返回所有事件
    pub async fn wait(mut self) -> Vec<AgentActorEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.event_rx.recv().await {
            let is_terminal = matches!(
                event,
                AgentActorEvent::Completed
                    | AgentActorEvent::Cancelled
                    | AgentActorEvent::Error(_)
                    | AgentActorEvent::MaxIterations { .. }
            );
            events.push(event);
            if is_terminal {
                break;
            }
        }
        events
    }
}

// ============================================================================
// 执行结果
// ============================================================================

/// 单步执行结果
#[derive(Debug, Clone)]
pub enum StepResult {
    /// 需要继续执行（有工具调用）
    Continue {
        content: String,
        reasoning_content: Option<String>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<CallToolResult>,
    },
    /// 执行完成（无工具调用）
    Done {
        content: String,
        reasoning_content: Option<String>,
    },
    /// 执行出错
    Error(AgentError),
}

// ============================================================================
// 循环状态
// ============================================================================

/// 循环执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopState {
    /// 正在运行
    Running,
    /// 已暂停
    Paused,
    /// 已取消
    Cancelled,
}

impl LoopState {
    /// 是否为终止状态
    fn is_terminal(self) -> bool {
        self == LoopState::Cancelled
    }
}

// ============================================================================
// Agent Actor
// ============================================================================

pub struct AgentActor<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// Agent 状态（控制面）
    state: AgentState,
    /// Chat 模型
    chat: Arc<C>,
    /// 工具执行器
    tool_executor: Arc<E>,
    /// 执行指标
    metrics: ExecutionMetrics,
    /// 步骤超时时间
    step_timeout: Option<Duration>,
    /// 工具执行超时时间
    tool_timeout: Option<Duration>,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
    /// 创建新的 Agent Actor
    pub fn new(chat: C, tool_executor: E, context: Context) -> Self {
        Self {
            state: AgentState {
                job_id: Uuid::new_v4(),
                user_id: "default".to_string(),
                conversation_id: None,
                title: String::new(),
                description: String::new(),
                category: None,
                state: crate::agents::JobState::Pending,
                budget: None,
                actual_cost: rust_decimal::Decimal::ZERO,
                context,
                iteration: 0,
                max_iterations: 10,
            },
            chat: Arc::new(chat),
            tool_executor: Arc::new(tool_executor),
            metrics: ExecutionMetrics::default(),
            step_timeout: None,
            tool_timeout: None,
        }
    }

    /// 获取状态
    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// 获取可变状态
    pub fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }

    /// 获取上下文
    pub fn context(&self) -> &Context {
        &self.state.context
    }

    /// 获取可变上下文
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.state.context
    }

    /// 获取执行指标
    pub fn metrics(&self) -> &ExecutionMetrics {
        &self.metrics
    }

    /// 发送事件（忽略发送失败）
    async fn send_event(event_tx: Option<&mpsc::Sender<AgentActorEvent>>, event: AgentActorEvent) {
        if let Some(tx) = event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// 执行单步迭代
    ///
    /// # Arguments
    /// * `event_tx` - 可选的事件发送器，用于报告执行过程中的各种事件
    ///
    /// # Returns
    /// 返回 `StepResult` 表示执行结果：
    /// - `Continue`: 有工具调用，需要继续迭代
    /// - `Done`: 无工具调用，执行完成
    /// - `Error`: 执行出错
    pub async fn run_step(
        &mut self,
        event_tx: Option<mpsc::Sender<AgentActorEvent>>,
    ) -> StepResult {
        let start = Instant::now();

        // 如果设置了超时，使用 timeout 包装
        let result = if let Some(timeout) = self.step_timeout {
            match tokio::time::timeout(timeout, self.run_step_inner(event_tx)).await {
                Ok(result) => result,
                Err(_) => {
                    self.state.state = crate::agents::JobState::Failed;
                    StepResult::Error(AgentError::Timeout)
                }
            }
        } else {
            self.run_step_inner(event_tx).await
        };

        // 更新指标
        self.metrics.total_duration += start.elapsed();

        result
    }

    /// 执行单步迭代的内部实现
    async fn run_step_inner(
        &mut self,
        event_tx: Option<mpsc::Sender<AgentActorEvent>>,
    ) -> StepResult {
        // 增加迭代计数，更新状态为 Running
        self.state.iteration += 1;
        self.metrics.iterations = self.state.iteration;
        self.state.state = crate::agents::JobState::Running;

        // 获取消息和工具定义
        let messages = self.state.context.to_messages();
        let tools_def = Some(self.tool_executor.tools().clone());

        // 调用模型并处理流
        let mut stream = pin!(call_model(
            &messages,
            tools_def.as_ref(),
            self.chat.as_ref()
        ));

        let mut content = String::new();
        let mut reasoning_content: Option<String> = None;
        let mut tool_calls: Option<Vec<ToolCall>> = None;
        let mut error: Option<String> = None;

        while let Some(event) = stream.next().await {
            match event {
                CallModelEvent::TextChunk(text) => {
                    content.push_str(&text);
                    Self::send_event(event_tx.as_ref(), AgentActorEvent::ContentChunk(text)).await;
                }
                CallModelEvent::ReasoningChunk(text) => {
                    reasoning_content
                        .get_or_insert_with(String::new)
                        .push_str(&text);
                    Self::send_event(event_tx.as_ref(), AgentActorEvent::ReasoningChunk(text))
                        .await;
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
                    error = Some(e);
                    break;
                }
            }
        }

        // 处理错误
        if let Some(e) = error {
            self.state.state = crate::agents::JobState::Failed;
            Self::send_event(
                event_tx.as_ref(),
                AgentActorEvent::Error(AgentError::Model(e.clone())),
            )
            .await;
            return StepResult::Error(AgentError::Model(e));
        }

        // 发送 StepCompleted 事件
        Self::send_event(
            event_tx.as_ref(),
            AgentActorEvent::StepCompleted {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: tool_calls.clone(),
            },
        )
        .await;

        // 根据是否有工具调用，选择不同的处理路径
        let result = if let Some(tc) = tool_calls {
            // 先发送 ToolCalls 事件
            Self::send_event(event_tx.as_ref(), AgentActorEvent::ToolCalls(tc.clone())).await;

            // 执行工具调用（带超时）
            let tool_results = self
                .execute_tools(&tc, event_tx.as_ref(), self.tool_timeout)
                .await;

            // 更新统计
            self.metrics.tool_calls_count += tc.len();

            // 保存用于返回的值（在移动前 clone）
            let tc_for_result = tc.clone();
            let content_for_result = content.clone();

            // 更新 Context
            self.state.context.add_message(Message::Assistant {
                content,
                reasoning_content: reasoning_content.clone(),
                tool_calls: Some(tc),
            });
            for tr in &tool_results {
                self.state.context.add_message(Message::Tool {
                    tool_call_id: tr.call_id.clone(),
                    content: tr.output.clone(),
                });
            }

            self.state.state = crate::agents::JobState::WaitingInput;
            StepResult::Continue {
                content: content_for_result,
                reasoning_content,
                tool_calls: tc_for_result,
                tool_results,
            }
        } else {
            // 更新 Context
            self.state.context.add_message(Message::Assistant {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: None,
            });

            self.state.state = crate::agents::JobState::Completed;
            StepResult::Done {
                content,
                reasoning_content,
            }
        };

        // 统一发送迭代完成事件
        Self::send_event(
            event_tx.as_ref(),
            AgentActorEvent::Iteration {
                iteration: self.state.iteration,
                message_count: self.state.context.conversation().len(),
            },
        )
        .await;

        result
    }

    /// 启动循环执行，返回控制句柄
    ///
    /// 此方法启动后台任务执行循环，直到完成或被打断。
    /// 返回的控制句柄可用于暂停、继续、取消等操作。
    ///
    /// 如果需要手动控制每一步执行，请使用 `run_step` 方法。
    pub fn run_loop(mut self) -> AgentActorHandle {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<AgentActorCommand>(16);
        let (event_tx, event_rx) = mpsc::channel::<AgentActorEvent>(64);

        tokio::spawn(async move {
            let mut loop_state = LoopState::Running;

            loop {
                // 处理所有待处理的命令
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        AgentActorCommand::Pause => {
                            loop_state = LoopState::Paused;
                            self.state.state = crate::agents::JobState::Paused;
                        }
                        AgentActorCommand::Continue => {
                            if loop_state == LoopState::Paused {
                                loop_state = LoopState::Running;
                                self.state.state = crate::agents::JobState::Running;
                            }
                        }
                        AgentActorCommand::Cancel => {
                            loop_state = LoopState::Cancelled;
                            self.state.state = crate::agents::JobState::Cancelled;
                        }
                    }
                }

                // 检查是否已取消
                if loop_state.is_terminal() {
                    Self::send_event(Some(&event_tx), AgentActorEvent::Cancelled).await;
                    break;
                }

                // 处理暂停状态
                if loop_state == LoopState::Paused {
                    tokio::select! {
                        Some(cmd) = cmd_rx.recv() => {
                            match cmd {
                                AgentActorCommand::Continue => {
                                    loop_state = LoopState::Running;
                                    self.state.state = crate::agents::JobState::Running;
                                }
                                AgentActorCommand::Cancel => {
                                    loop_state = LoopState::Cancelled;
                                    self.state.state = crate::agents::JobState::Cancelled;
                                }
                                AgentActorCommand::Pause => {
                                    // 已经暂停，忽略重复的暂停命令
                                }
                            }
                        }
                        _ = event_tx.closed() => {
                            // receiver 被关闭，退出
                            loop_state = LoopState::Cancelled;
                        }
                    }
                    continue;
                }

                // 执行前检查最大迭代
                if self.state.iteration >= self.state.max_iterations {
                    Self::send_event(
                        Some(&event_tx),
                        AgentActorEvent::MaxIterations {
                            iteration: self.state.iteration,
                        },
                    )
                    .await;
                    Self::send_event(
                        Some(&event_tx),
                        AgentActorEvent::Error(AgentError::MaxIterations(self.state.iteration)),
                    )
                    .await;
                    break;
                }

                // 执行单步迭代
                let result = self.run_step(Some(event_tx.clone())).await;

                // 根据结果决定后续行为
                match result {
                    StepResult::Continue { .. } => {
                        // 继续下一次迭代
                    }
                    StepResult::Done { .. } => {
                        Self::send_event(Some(&event_tx), AgentActorEvent::Completed).await;
                        break;
                    }
                    StepResult::Error(e) => {
                        Self::send_event(Some(&event_tx), AgentActorEvent::Error(e)).await;
                        break;
                    }
                }
            }
        });

        AgentActorHandle { cmd_tx, event_rx }
    }

    /// 执行工具调用并收集结果
    ///
    /// # Arguments
    /// * `tool_calls` - 要执行的工具调用列表
    /// * `event_tx` - 可选的事件发送器，用于报告每个工具的执行结果
    /// * `timeout` - 可选的超时时间
    ///
    /// # Returns
    /// 返回所有工具调用的结果列表
    async fn execute_tools(
        &self,
        tool_calls: &[ToolCall],
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        timeout: Option<Duration>,
    ) -> Vec<CallToolResult> {
        let execute = async {
            let mut stream = pin!(call_tools(self.tool_executor.as_ref(), tool_calls));
            let mut results = Vec::new();
            while let Some(result) = stream.next().await {
                Self::send_event(
                    event_tx,
                    AgentActorEvent::ToolResult {
                        call_id: result.call_id.clone(),
                        success: result.success,
                        output: result.output.clone(),
                    },
                )
                .await;
                results.push(result);
            }
            results
        };

        // 如果设置了超时，使用 timeout 包装
        if let Some(dur) = timeout {
            match tokio::time::timeout(dur, execute).await {
                Ok(results) => results,
                Err(_) => {
                    // 超时时发送错误事件
                    Self::send_event(event_tx, AgentActorEvent::Error(AgentError::Timeout)).await;
                    Vec::new()
                }
            }
        } else {
            execute.await
        }
    }
}

// ============================================================================
// Agent Actor Builder
// ============================================================================

/// Agent Actor 构建器
pub struct AgentActorBuilder<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    chat: C,
    tool_executor: E,
    context: Context,
    max_iterations: usize,
    user_id: String,
    step_timeout: Option<Duration>,
    tool_timeout: Option<Duration>,
}

impl<C, E> AgentActorBuilder<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// 创建新的构建器
    pub fn new(chat: C, tool_executor: E) -> Self {
        Self {
            chat,
            tool_executor,
            context: Context::new(),
            max_iterations: 10,
            user_id: "default".to_string(),
            step_timeout: None,
            tool_timeout: None,
        }
    }

    /// 设置上下文
    pub fn context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    /// 设置最大迭代次数
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// 设置用户 ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = user_id.into();
        self
    }

    /// 设置步骤超时时间
    pub fn step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = Some(timeout);
        self
    }

    /// 设置工具执行超时时间
    pub fn tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = Some(timeout);
        self
    }

    /// 构建 AgentActor
    pub fn build(self) -> AgentActor<C, E> {
        let mut actor = AgentActor::new(self.chat, self.tool_executor, self.context);
        actor.state.max_iterations = self.max_iterations;
        actor.state.user_id = self.user_id;
        actor.step_timeout = self.step_timeout;
        actor.tool_timeout = self.tool_timeout;
        actor
    }
}
