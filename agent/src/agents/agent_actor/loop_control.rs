use std::sync::Arc;

use tokio::sync::mpsc;

use super::lifecycle::StepLifeCycle;
use super::{AgentActor, AgentActorCommand, AgentActorEvent, AgentActorHandle, StepResult};
use crate::agents::agent_actor::lifecycle::StepFrame;
use crate::agents::{AgentError, JobState, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

enum LoopState {
    Runnable,
    Paused,
    Finished(Finalization),
}

enum Finalization {
    Emit(TerminalReason),
    Silent,
}

enum TerminalReason {
    Completed,
    Cancelled,
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
        let chat = Arc::clone(&self.chat);
        let tool_executor = Arc::clone(&self.tool_executor);
        let mut lifecycle = StepLifeCycle::new(self.state.clone());

        // 执行生命周期函数
        let frame = match lifecycle.start(chat.as_ref(), tool_executor.as_ref()).await {
            Ok(frame) => frame,
            Err(err) => return StepResult::Error(err.into()),
        };
        self.apply_step_usage(&frame);
        let result = Self::step_result_from_frame(frame);
        // 应用 StepFrame 结果，更新 AgentState
        self.apply_step_result(&result);
        // 对外发送 StepResult 事件
        self.emit_step_result_events(event_tx.as_ref(), &result)
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
                LoopState::Finished(Finalization::Emit(TerminalReason::Cancelled))
            }
        }
    }

    // 两种改变 Agent 执行状态的方法
    // 1. 接受外部 Command 改变内部运行状态
    fn transition_command(&mut self, loop_state: LoopState, cmd: AgentActorCommand) -> LoopState {
        match (loop_state, cmd) {
            (LoopState::Runnable, AgentActorCommand::Pause) => {
                self.state.state = JobState::Paused;
                LoopState::Paused
            }
            (LoopState::Runnable, AgentActorCommand::Continue) => LoopState::Runnable,
            (LoopState::Runnable, AgentActorCommand::Cancel) => {
                self.state.state = JobState::Cancelled;
                LoopState::Finished(Finalization::Emit(TerminalReason::Cancelled))
            }
            (LoopState::Paused, AgentActorCommand::Pause) => LoopState::Paused,
            (LoopState::Paused, AgentActorCommand::Continue) => {
                self.state.state = JobState::Running;
                LoopState::Runnable
            }
            (LoopState::Paused, AgentActorCommand::Cancel) => {
                self.state.state = JobState::Cancelled;
                LoopState::Finished(Finalization::Emit(TerminalReason::Cancelled))
            }
            (LoopState::Finished(finalization), _) => LoopState::Finished(finalization),
        }
    }

    // 2. 根据单步执行的结果改变运行状态
    fn transition_step_result(&self, result: StepResult) -> LoopState {
        match result {
            StepResult::Continue { .. } => LoopState::Runnable,
            StepResult::Done { .. } => {
                LoopState::Finished(Finalization::Emit(TerminalReason::Completed))
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
                TerminalReason::Completed => {
                    Self::send_event(Some(event_tx), AgentActorEvent::Completed).await;
                }
                TerminalReason::Cancelled => {
                    Self::send_event(Some(event_tx), AgentActorEvent::Cancelled).await;
                }
            },
            Finalization::Silent => {}
        }
    }

    fn apply_step_result(&mut self, result: &StepResult) {
        self.state.iteration += 1;

        match result {
            StepResult::Continue {
                content,
                reasoning_content,
                tools_call,
                tools_result,
            } => {
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
                self.state.state = JobState::Completed;
                self.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: None,
                });
            }
            StepResult::Error(err) => {
                self.state.state = match err {
                    AgentError::Cancelled => JobState::Cancelled,
                    _ => JobState::Failed,
                };
            }
        }
    }

    fn apply_step_usage(&mut self, frame: &StepFrame) {
        if let Some(usage) = frame.token_usage.as_ref() {
            self.state.token_statistics.add_usage(usage);
        }
    }

    fn step_result_from_frame(frame: StepFrame) -> StepResult {
        let StepFrame {
            model_output,
            tools_result,
            ..
        } = frame;

        match model_output {
            Some(model_output) => match model_output.tools_call {
                Some(tools_call) => StepResult::Continue {
                    content: model_output.content,
                    reasoning_content: model_output.reasoning_content,
                    tools_call,
                    tools_result: tools_result.unwrap_or_default(),
                },
                None => StepResult::Done {
                    content: model_output.content,
                    reasoning_content: model_output.reasoning_content,
                },
            },
            None => StepResult::Error(AgentError::ModelRspErr),
        }
    }

    async fn emit_step_result_events(
        &self,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        result: &StepResult,
    ) {
        Self::send_event(
            event_tx,
            AgentActorEvent::StepFinalized {
                result: result.clone(),
            },
        )
        .await;

        match result {
            StepResult::Continue { .. } | StepResult::Done { .. } => {
                Self::send_event(
                    event_tx,
                    AgentActorEvent::Iteration {
                        iteration: self.state.iteration,
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
}
