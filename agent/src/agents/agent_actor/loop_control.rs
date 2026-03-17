use tokio::sync::mpsc;

use super::{AgentActor, AgentActorCommand, AgentActorEvent, AgentActorHandle, StepResult};
use crate::agents::ToolExecutor;
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
    fn set_loop_state(&mut self, loop_state: &mut LoopState, next_state: LoopState) {
        *loop_state = next_state;
        self.state.state = match next_state {
            LoopState::Running => crate::agents::JobState::Running,
            LoopState::Paused => crate::agents::JobState::Paused,
            LoopState::Cancelled => crate::agents::JobState::Cancelled,
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

    async fn handle_step_result(
        event_tx: &mpsc::Sender<AgentActorEvent>,
        result: StepResult,
    ) -> bool {
        match result {
            StepResult::Continue { .. } => true,
            StepResult::Done { .. } => {
                Self::send_event(Some(event_tx), AgentActorEvent::Completed).await;
                false
            }
            StepResult::Error(_) => false,
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
            let mut loop_state = LoopState::Running;

            loop {
                if event_tx.is_closed() {
                    break;
                }

                self.drain_commands(&mut cmd_rx, &mut loop_state);

                if loop_state.is_terminal() {
                    Self::send_event(Some(&event_tx), AgentActorEvent::Cancelled).await;
                    break;
                }

                self.wait_if_paused(&mut cmd_rx, &event_tx, &mut loop_state)
                    .await;

                if loop_state == LoopState::Paused {
                    continue;
                }

                let result = self.run_step(Some(event_tx.clone())).await;
                if !Self::handle_step_result(&event_tx, result).await {
                    break;
                }
            }
        });

        AgentActorHandle { cmd_tx, event_rx }
    }
}
