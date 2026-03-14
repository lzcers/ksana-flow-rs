//! Actor 控制器 - 命令处理、状态管理、事件发送、迭代控制
//!
//! Actor 是一个独立执行单元，具有以下特性：
//! - 可插拔的 Chat 模型和 ToolExecutor
//! - 支持暂停、继续、取消操作
//! - 状态可持久化和恢复
//! - 流式事件输出
//!
//! 职责分离：
//! - 执行逻辑由 `AgentExecutor` 处理（单步执行）
//! - 本文件负责控制面：命令处理、状态转换、事件发送、迭代控制

use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use super::agent_executor::{AgentExecutor, StepResult, StreamChunk};
use crate::agents::{AgentState, Context, ToolCall, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

// ============================================================================
// Actor 事件
// ============================================================================

/// Actor 产生的事件
#[derive(Debug, Clone)]
pub enum AgentActorEvent {
    /// 流式输出片段
    Chunk(String),
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
    /// 发生错误
    Error(String),
}

// ============================================================================
// Actor 命令
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
// Actor Handle
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
}

// ============================================================================
// Agent Actor
// ============================================================================

/// Agent Actor - 可插拔的 LLM Agent 控制器
///
/// 泛型参数：
/// - `C`: Chat 模型能力
/// - `E`: 工具执行器
///
/// Actor 负责控制面逻辑：
/// - 命令处理（暂停、继续、取消、单步）
/// - 状态管理（JobState、迭代计数、Context）
/// - 事件发送
/// - 迭代控制（最大迭代次数）
///
/// 执行面由 `AgentExecutor` 处理，只负责单步执行。
/// Context 由 Actor 管理，执行器不持有状态。
pub struct AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send + Sync,
{
    /// Agent 状态（控制面）
    state: AgentState,
    /// 执行器（执行面，无状态）
    executor: AgentExecutor<C, E>,
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
                state: crate::agents::JobState::Pending,
                budget: None,
                actual_cost: rust_decimal::Decimal::ZERO,
                context,
                iteration: 0,
                max_iterations: 10,
            },
            executor: AgentExecutor::new(chat, executor),
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

    pub fn context(&self) -> &Context {
        &self.state.context
    }
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.state.context
    }
    /// 执行单步迭代
    ///
    /// 此方法执行一次 LLM 调用 + 工具执行，然后返回结果。
    /// 可以重复调用来手动控制执行流程。
    ///
    /// # 参数
    /// - `stream_tx`: 可选的流式事件发送通道，用于接收 `StreamChunk` 事件
    ///
    /// # 返回
    /// - `StepResult`: 执行结果
    ///
    /// # 状态更新
    /// - 自动增加迭代计数
    /// - 更新 Context（添加 assistant 消息和 tool 消息）
    /// - 更新 JobState
    ///
    /// # 示例
    /// ```ignore
    /// let mut actor = AgentActor::new(chat, executor, context);
    /// loop {
    ///     let result = actor.run_step(Some(stream_tx.clone())).await;
    ///     match result {
    ///         StepResult::Done { .. } => break,
    ///         StepResult::Error(_) => break,
    ///         StepResult::Continue { .. } => continue,
    ///     }
    /// }
    /// ```
    pub async fn run_step(&mut self, stream_tx: Option<mpsc::Sender<StreamChunk>>) -> StepResult {
        // 增加迭代计数
        self.state.iteration += 1;

        // 更新状态为 Running
        self.state.state = crate::agents::JobState::Running;

        // 从 Context 获取消息列表
        let messages = self.state.context.to_messages();

        // 执行单步迭代
        let result = self.executor.run_step(messages, stream_tx).await;

        // 处理执行结果
        match &result {
            StepResult::Continue {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            } => {
                // 更新 Context：添加 assistant 消息
                self.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                });

                // 更新 Context：添加 tool 结果消息
                for tr in tool_results {
                    self.state.context.add_message(Message::Tool {
                        tool_call_id: tr.call_id.clone(),
                        content: tr.output.clone(),
                    });
                }

                // 继续状态
                self.state.state = crate::agents::JobState::WaitingInput;
            }
            StepResult::Done {
                content,
                reasoning_content,
            } => {
                // 更新 Context：添加 assistant 消息
                self.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: None,
                });

                // 完成状态
                self.state.state = crate::agents::JobState::Completed;
            }
            StepResult::Error(_) => {
                self.state.state = crate::agents::JobState::Failed;
            }
        }

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
            let mut paused = false;
            let mut cancelled = false;

            loop {
                // 检查命令
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        AgentActorCommand::Pause => {
                            paused = true;
                            self.state.state = crate::agents::JobState::Paused;
                        }
                        AgentActorCommand::Continue => {
                            paused = false;
                            if self.state.state == crate::agents::JobState::Paused {
                                self.state.state = crate::agents::JobState::Running;
                            }
                        }
                        AgentActorCommand::Cancel => {
                            cancelled = true;
                            self.state.state = crate::agents::JobState::Cancelled;
                        }
                    }
                }

                if cancelled {
                    let _ = event_tx
                        .send(AgentActorEvent::Error("Cancelled".to_string()))
                        .await;
                    break;
                }

                if paused {
                    // 等待继续命令
                    if let Some(AgentActorCommand::Continue) = cmd_rx.recv().await {
                        paused = false;
                        self.state.state = crate::agents::JobState::Running;
                    }
                    continue;
                }

                // 执行前检查最大迭代
                if self.state.iteration >= self.state.max_iterations {
                    let _ = event_tx
                        .send(AgentActorEvent::MaxIterations {
                            iteration: self.state.iteration,
                        })
                        .await;
                    let _ = event_tx
                        .send(AgentActorEvent::Error("Max iterations reached".to_string()))
                        .await;
                    break;
                }

                // 创建 StreamChunk channel，在后台转换为 AgentActorEvent
                let (stream_tx, mut stream_rx) = mpsc::channel::<StreamChunk>(64);
                let event_tx_clone = event_tx.clone();

                // 后台任务：将 StreamChunk 转换为 AgentActorEvent
                let convert_handle = tokio::spawn(async move {
                    while let Some(chunk) = stream_rx.recv().await {
                        match chunk {
                            StreamChunk::Text(text) => {
                                if event_tx_clone
                                    .send(AgentActorEvent::Chunk(text))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            StreamChunk::Reasoning(text) => {
                                if event_tx_clone
                                    .send(AgentActorEvent::ReasoningChunk(text))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            StreamChunk::Completed {
                                content,
                                reasoning_content,
                                tool_calls,
                            } => {
                                if event_tx_clone
                                    .send(AgentActorEvent::StepCompleted {
                                        content,
                                        reasoning_content,
                                        tool_calls,
                                    })
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                });

                // === 执行单步迭代 ===
                let result = self.run_step(Some(stream_tx)).await;

                // 等待转换任务完成
                let _ = convert_handle.await;

                // 发送工具相关事件
                match &result {
                    StepResult::Continue {
                        tool_calls,
                        tool_results,
                        ..
                    } => {
                        let _ = event_tx
                            .send(AgentActorEvent::ToolCalls(tool_calls.clone()))
                            .await;
                        for tr in tool_results {
                            let _ = event_tx
                                .send(AgentActorEvent::ToolResult {
                                    call_id: tr.call_id.clone(),
                                    success: tr.success,
                                    output: tr.output.clone(),
                                })
                                .await;
                        }
                        let _ = event_tx
                            .send(AgentActorEvent::Iteration {
                                iteration: self.state.iteration,
                                message_count: self.state.context.conversation().len(),
                            })
                            .await;
                    }
                    StepResult::Done { .. } => {
                        let _ = event_tx
                            .send(AgentActorEvent::Iteration {
                                iteration: self.state.iteration,
                                message_count: self.state.context.conversation().len(),
                            })
                            .await;
                        let _ = event_tx.send(AgentActorEvent::Completed).await;
                    }
                    StepResult::Error(e) => {
                        let _ = event_tx.send(AgentActorEvent::Error(e.clone())).await;
                    }
                }

                // 根据结果决定后续行为
                match result {
                    StepResult::Continue { .. } => {
                        // 循环模式直接继续下一次迭代
                    }
                    StepResult::Done { .. } | StepResult::Error(_) => {
                        // 完成或错误，退出循环
                        break;
                    }
                }
            }
        });

        AgentActorHandle { cmd_tx, event_rx }
    }
}
