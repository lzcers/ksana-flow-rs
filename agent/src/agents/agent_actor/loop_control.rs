use std::sync::Arc;

use tokio::sync::mpsc;

use super::lifecycle::StepLifeCycle;
use super::{AgentActor, AgentActorCommand, AgentActorEvent, AgentActorHandle, StepResult};
use crate::agents::agent_actor::lifecycle::StepFrame;
use crate::agents::{AgentError, JobState, ToolExecutor};
use crate::core::Message;
use crate::models::ChatCapability;

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
        let result = match lifecycle.start(chat.as_ref(), tool_executor.as_ref()).await {
            Ok(frame) => Self::step_result_from_frame(frame),
            Err(err) => StepResult::Error(err.into()),
        };

        self.apply_step_result(&result);
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
            let mut loop_state = LoopState::Running;

            loop {
                if event_tx.is_closed() {
                    break;
                }

                self.drain_commands(&mut cmd_rx, &mut loop_state);

                if Self::break_if_cancelled(&event_tx, loop_state).await {
                    break;
                }

                self.wait_if_paused(&mut cmd_rx, &event_tx, &mut loop_state)
                    .await;

                if Self::break_if_cancelled(&event_tx, loop_state).await {
                    break;
                }

                if loop_state == LoopState::Paused {
                    continue;
                }

                match self.run_step(Some(event_tx.clone())).await {
                    StepResult::Continue { .. } => continue,
                    StepResult::Done { .. } => {
                        Self::send_event(Some(&event_tx), AgentActorEvent::Completed).await;
                        break;
                    }
                    StepResult::Error(_) => {
                        break;
                    }
                }
            }
        });

        AgentActorHandle { cmd_tx, event_rx }
    }

    fn set_loop_state(&mut self, loop_state: &mut LoopState, next_state: LoopState) {
        *loop_state = next_state;
        self.state.state = match next_state {
            LoopState::Running => JobState::Running,
            LoopState::Paused => JobState::Paused,
            LoopState::Cancelled => JobState::Cancelled,
        };
    }

    fn apply_command(&mut self, loop_state: &mut LoopState, cmd: AgentActorCommand) {
        match cmd {
            AgentActorCommand::Pause => self.set_loop_state(loop_state, LoopState::Paused),
            AgentActorCommand::Continue => {
                if *loop_state == LoopState::Paused {
                    self.set_loop_state(loop_state, LoopState::Running);
                }
            }
            AgentActorCommand::Cancel => self.set_loop_state(loop_state, LoopState::Cancelled),
        }
    }

    fn drain_commands(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<AgentActorCommand>,
        loop_state: &mut LoopState,
    ) {
        while let Ok(cmd) = cmd_rx.try_recv() {
            self.apply_command(loop_state, cmd);
        }
    }

    async fn wait_if_paused(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<AgentActorCommand>,
        event_tx: &mpsc::Sender<AgentActorEvent>,
        loop_state: &mut LoopState,
    ) {
        if *loop_state != LoopState::Paused {
            return;
        }

        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                self.apply_command(loop_state, cmd);
            }
            _ = event_tx.closed() => {
                self.set_loop_state(loop_state, LoopState::Cancelled);
            }
        }
    }

    async fn break_if_cancelled(
        event_tx: &mpsc::Sender<AgentActorEvent>,
        loop_state: LoopState,
    ) -> bool {
        if loop_state.is_terminal() {
            Self::send_event(Some(event_tx), AgentActorEvent::Cancelled).await;
            true
        } else {
            false
        }
    }

    fn step_result_from_frame(frame: StepFrame) -> StepResult {
        let StepFrame {
            model_output,
            tools_result,
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
