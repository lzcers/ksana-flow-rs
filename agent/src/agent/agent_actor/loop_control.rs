use std::sync::Arc;

use tokio::sync::mpsc;

use super::lifecycle::{LifeCycleFlow, LifeCycleInterrupt, LifeCycleResult, StepLifeCycle};
use super::{AgentActor, AgentActorCommand, AgentActorEvent, AgentActorHandle, StepResult};
use crate::agent::agent_actor::lifecycle::StepFrame;
use crate::agent::{AgentError, AgentTerminalReason, JobState, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

#[derive(Clone)]
pub enum LoopState {
    Runnable,
    Paused,
    WaitingForUserInput(String),
    Finished(Finalization),
}

#[derive(Clone)]
pub enum Finalization {
    Emit(AgentTerminalReason),
    Silent,
}

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
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
        self.state.state = JobState::Running;
        self.state.metrics.mark_started();
        let chat = Arc::clone(&self.chat);
        let tool_executor = Arc::clone(&self.tool_executor);
        let mut lifecycle = StepLifeCycle::new(self.state.clone());

        // 执行生命周期函数
        let lifecycle_flow = lifecycle
            .start(chat.as_ref(), tool_executor.as_ref(), event_tx.as_ref())
            .await;

        match lifecycle_flow {
            LifeCycleFlow::Continue(LifeCycleResult::Frame(frame)) => {
                self.apply_step_metrics(&frame);
                let result = Self::step_result_from_frame(&frame);
                // 应用 StepFrame 结果，更新 AgentState
                self.apply_step_result(&result);
                // 对外发送 StepResult 事件
                self.emit_step_result_events(event_tx.as_ref(), &frame, &result)
                    .await;
                result
            }
            LifeCycleFlow::Break(interrupt) => {
                match interrupt {
                    // 如果是 AskUser 中断，返回成功结果
                    LifeCycleInterrupt::AskUser { question } => {
                        // 触发用户输入请求，进入 WaitingForUserInput 状态
                        self.ask_user(question, event_tx).await;
                        // 返回 Continue 结果，让 loop 继续处理状态转换
                        // 注意：这里我们没有完整的 model_output，但这不影响状态转换
                        StepResult::Continue {
                            content: String::new(),
                            reasoning_content: None,
                            tools_call: Vec::new(),
                            tools_result: Vec::new(),
                        }
                    }
                    // 其他中断（错误）返回错误
                    _ => {
                        // 将 LifeCycleInterrupt 转换为 AgentError
                        let err = AgentError::Parse(interrupt);
                        self.apply_step_error(&err);
                        self.emit_error_events(event_tx.as_ref(), &err).await;
                        StepResult::Error(err)
                    }
                }
            }
            _ => {
                // 其他情况视为模型错误
                let err = AgentError::Parse(LifeCycleInterrupt::ModelError);
                self.apply_step_error(&err);
                self.emit_error_events(event_tx.as_ref(), &err).await;
                StepResult::Error(err)
            }
        }
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
            let mut loop_state = LoopState::Runnable;

            loop {
                if event_tx.is_closed() {
                    break;
                }

                loop_state = self.drain_commands(loop_state, &mut cmd_rx);

                let current_state = std::mem::replace(&mut loop_state, LoopState::Runnable);
                match current_state {
                    LoopState::Runnable => {
                        let result = self.run_step(Some(event_tx.clone())).await;
                        loop_state = self.transition_step_result(result);
                    }
                    LoopState::Paused => {
                        loop_state = self.wait_while_paused(&mut cmd_rx, &event_tx).await;
                    }
                    LoopState::WaitingForUserInput(_) => {
                        loop_state = self.wait_for_user_input(&mut cmd_rx, &event_tx).await;
                    }
                    LoopState::Finished(finalization) => {
                        self.emit_terminal_event(&event_tx, &finalization).await;
                        break;
                    }
                };

                if let LoopState::Finished(finalization) = &loop_state {
                    self.emit_terminal_event(&event_tx, finalization).await;
                    break;
                }
            }
        });

        AgentActorHandle { cmd_tx, event_rx }
    }

    fn drain_commands(
        &mut self,
        mut loop_state: LoopState,
        cmd_rx: &mut mpsc::Receiver<AgentActorCommand>,
    ) -> LoopState {
        while let Ok(cmd) = cmd_rx.try_recv() {
            loop_state = self.transition_command(loop_state, cmd);
        }
        loop_state
    }

    async fn wait_while_paused(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<AgentActorCommand>,
        event_tx: &mpsc::Sender<AgentActorEvent>,
    ) -> LoopState {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                self.transition_command(LoopState::Paused, cmd)
            }
            _ = event_tx.closed() => {
                self.state.state = JobState::Cancelled;
                self.state.metrics.mark_finished(AgentTerminalReason::Cancelled);
                LoopState::Finished(Finalization::Emit(AgentTerminalReason::Cancelled))
            }
        }
    }

    async fn wait_for_user_input(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<AgentActorCommand>,
        event_tx: &mpsc::Sender<AgentActorEvent>,
    ) -> LoopState {
        if let Some((input_id, _)) = &self.pending_user_input {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    self.transition_command(LoopState::WaitingForUserInput(input_id.clone()), cmd)
                }
                _ = event_tx.closed() => {
                    self.state.state = JobState::Cancelled;
                    self.state.metrics.mark_finished(AgentTerminalReason::Cancelled);
                    self.pending_user_input = None;
                    LoopState::Finished(Finalization::Emit(AgentTerminalReason::Cancelled))
                }
            }
        } else {
            LoopState::Runnable
        }
    }

    // 两种改变 Agent 执行状态的方法
    // 1. 接受外部 Command 改变内部运行状态
    fn transition_command(&mut self, loop_state: LoopState, cmd: AgentActorCommand) -> LoopState {
        match loop_state {
            LoopState::Runnable => match cmd {
                AgentActorCommand::Pause => {
                    self.state.state = JobState::Paused;
                    self.state.metrics.mark_active();
                    LoopState::Paused
                }
                AgentActorCommand::Continue => {
                    self.state.metrics.mark_active();
                    LoopState::Runnable
                }
                AgentActorCommand::Cancel => {
                    self.state.state = JobState::Cancelled;
                    self.state
                        .metrics
                        .mark_finished(AgentTerminalReason::Cancelled);
                    LoopState::Finished(Finalization::Emit(AgentTerminalReason::Cancelled))
                }
                AgentActorCommand::UserInput { .. } => LoopState::Runnable,
            },
            LoopState::Paused => match cmd {
                AgentActorCommand::Pause => LoopState::Paused,
                AgentActorCommand::Continue => {
                    self.state.state = JobState::Running;
                    self.state.metrics.mark_active();
                    LoopState::Runnable
                }
                AgentActorCommand::Cancel => {
                    self.state.state = JobState::Cancelled;
                    self.state
                        .metrics
                        .mark_finished(AgentTerminalReason::Cancelled);
                    LoopState::Finished(Finalization::Emit(AgentTerminalReason::Cancelled))
                }
                AgentActorCommand::UserInput { .. } => LoopState::Paused,
            },
            LoopState::WaitingForUserInput(expected_input_id) => match cmd {
                AgentActorCommand::UserInput { input, input_id } => {
                    if input_id == expected_input_id {
                        self.state
                            .context
                            .add_message(Message::User { content: input });
                        self.state.state = JobState::Running;
                        self.pending_user_input = None;
                        LoopState::Runnable
                    } else {
                        LoopState::WaitingForUserInput(expected_input_id)
                    }
                }
                AgentActorCommand::Cancel => {
                    self.state.state = JobState::Cancelled;
                    self.state
                        .metrics
                        .mark_finished(AgentTerminalReason::Cancelled);
                    self.pending_user_input = None;
                    LoopState::Finished(Finalization::Emit(AgentTerminalReason::Cancelled))
                }
                _ => LoopState::WaitingForUserInput(expected_input_id),
            },
            LoopState::Finished(finalization) => LoopState::Finished(finalization),
        }
    }

    // 2. 根据单步执行的结果改变运行状态
    fn transition_step_result(&self, result: StepResult) -> LoopState {
        match result {
            StepResult::Continue { tools_call, .. } => {
                // 检查是否有 AskUser 工具调用
                if tools_call.iter().any(|tool| tool.get_name() == "ask_user") {
                    // 如果有 AskUser 工具调用，检查是否已经设置了 pending_user_input
                    if self.has_pending_user_input() {
                        // 从 pending_user_input 中获取 input_id
                        if let Some((input_id, _)) = &self.pending_user_input {
                            LoopState::WaitingForUserInput(input_id.clone())
                        } else {
                            LoopState::Runnable
                        }
                    } else {
                        LoopState::Runnable
                    }
                } else {
                    LoopState::Runnable
                }
            }
            StepResult::Done { .. } => {
                LoopState::Finished(Finalization::Emit(AgentTerminalReason::Completed))
            }
            StepResult::Error(_) => LoopState::Finished(Finalization::Silent),
        }
    }

    async fn emit_terminal_event(
        &self,
        event_tx: &mpsc::Sender<AgentActorEvent>,
        finalization: &Finalization,
    ) {
        match finalization {
            Finalization::Emit(reason) => match reason {
                AgentTerminalReason::Completed => {
                    Self::send_event(Some(event_tx), AgentActorEvent::Completed).await;
                }
                AgentTerminalReason::Cancelled => {
                    Self::send_event(Some(event_tx), AgentActorEvent::Cancelled).await;
                }
                AgentTerminalReason::Failed => {}
            },
            Finalization::Silent => {}
        }
    }

    fn apply_step_result(&mut self, result: &StepResult) {
        match result {
            StepResult::Continue {
                content,
                reasoning_content,
                tools_call,
                tools_result,
            } => {
                self.state.metrics.increment_iteration();
                self.state.state = JobState::Running;
                self.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: Some(tools_call.clone()),
                });

                for tool_result in tools_result {
                    self.state.context.add_message(Message::Tool {
                        tool_call_id: tool_result.call_id.clone(),
                        content: tool_result.output.clone(),
                    });
                }
            }
            StepResult::Done {
                content,
                reasoning_content,
            } => {
                self.state.metrics.increment_iteration();
                self.state.state = JobState::Completed;
                self.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: None,
                });
                self.state
                    .metrics
                    .mark_finished(AgentTerminalReason::Completed);
            }
            StepResult::Error(err) => self.apply_step_error(err),
        }
    }

    fn apply_step_metrics(&mut self, frame: &StepFrame) {
        self.state.metrics.mark_active();
        if let Some(usage) = frame.token_usage.as_ref() {
            self.state.metrics.add_usage(usage);
        }
        if let Some(duration_ms) = frame.metrics.call_model_duration_ms {
            self.state.metrics.add_model_duration(duration_ms);
        }
        if let Some(duration_ms) = frame.metrics.call_tools_duration_ms {
            self.state.metrics.add_tool_duration(duration_ms);
        }
        if let Some(results) = frame.tools_result.as_ref() {
            let success_count = results.iter().filter(|result| result.success).count();
            let failure_count = results.len().saturating_sub(success_count);
            self.state
                .metrics
                .add_tool_results(success_count, failure_count);
        }
    }

    fn apply_step_error(&mut self, err: &AgentError) {
        match err {
            AgentError::Cancelled => {
                self.state.state = JobState::Cancelled;
                self.state
                    .metrics
                    .mark_finished(AgentTerminalReason::Cancelled);
            }
            _ => {
                self.state.state = JobState::Failed;
                self.state.metrics.record_error(err.to_string());
                self.state
                    .metrics
                    .mark_finished(AgentTerminalReason::Failed);
            }
        }
    }

    fn step_result_from_frame(frame: &StepFrame) -> StepResult {
        match frame.model_output.as_ref() {
            Some(model_output) => match model_output.tools_call.as_ref() {
                Some(tools_call) => StepResult::Continue {
                    content: model_output.content.clone(),
                    reasoning_content: model_output.reasoning_content.clone(),
                    tools_call: tools_call.clone(),
                    tools_result: frame.tools_result.clone().unwrap_or_default(),
                },
                None => StepResult::Done {
                    content: model_output.content.clone(),
                    reasoning_content: model_output.reasoning_content.clone(),
                },
            },
            None => StepResult::Error(AgentError::ModelRspErr),
        }
    }

    async fn emit_step_result_events(
        &self,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        frame: &StepFrame,
        result: &StepResult,
    ) {
        Self::send_event(
            event_tx,
            AgentActorEvent::StepFinalized {
                result: result.clone(),
                frame: frame.clone(),
            },
        )
        .await;

        match result {
            StepResult::Continue { .. } | StepResult::Done { .. } => {
                Self::send_event(
                    event_tx,
                    AgentActorEvent::Iteration {
                        iteration: self.state.metrics.execution.iteration,
                        message_count: self.state.context.conversation().len(),
                    },
                )
                .await;
            }
            StepResult::Error(AgentError::Cancelled) => {
                Self::send_event(event_tx, AgentActorEvent::Cancelled).await;
            }
            StepResult::Error(err) => {
                Self::send_event(event_tx, AgentActorEvent::Error(err.clone())).await;
            }
        }
    }

    async fn emit_error_events(
        &self,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        err: &AgentError,
    ) {
        match err {
            AgentError::Cancelled => {
                Self::send_event(event_tx, AgentActorEvent::Cancelled).await;
            }
            _ => {
                Self::send_event(event_tx, AgentActorEvent::Error(err.clone())).await;
            }
        }
    }
}
