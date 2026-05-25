use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use serde::Serialize;
use tokio::{
    sync::{broadcast, mpsc},
    time::{Duration, sleep},
};

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        export::{ExportHandle, ExportState, SessionSnapshot, TaskEvent, TaskView},
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PendingProtagonistChoice, ProtagonistDecisionState},
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    systems::{
        export_sys::export_system,
        fate_weaver_sys::{fate_weaver_apply_system, fate_weaver_system},
        player_input_sys::player_input_system,
        protagonist_sys::{protagonist_apply_system, protagonist_system},
        task_sys::task_system,
        turn_orchestrator_sys::turn_orchestrator_system,
        upper_narrator_sys::{upper_narrator_apply_system, upper_narrator_system},
    },
    turn_messages::{PlayerCommand, TurnControl, TurnEvent},
    utils::build_chat_model,
};

const DEFAULT_RUNTIME_TICK_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub phase: TurnPhase,                       // 阶段
    pub turn_index: u64,                        // 轮次
    pub active_turn_id: u64,                    // 当前轮次ID
    pub current_task: Option<TaskView>,         // 当前任务
    pub tasks: Vec<TaskView>,                   // 任务列表
    pub world_snapshot: WorldSnapshot,          // 世界快照
    pub latest_narration: String,               // 最新叙事
    pub current_protagonist_action: String,     // 当前 protagonist 动作
    pub choices: Vec<PendingProtagonistChoice>, // 待处理选择
}

pub struct AkashicSessionEngine {
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    export_handle: ExportHandle,
}

impl AkashicSessionEngine {
    // 创建实例
    pub fn new() -> Self {
        Self::new_with_profiles(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE)
    }

    pub fn new_with_profiles(world_profile: &str, protagonist_profile: &str) -> Self {
        let (world, export_handle) = build_world(world_profile, protagonist_profile);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        SessionRuntime::spawn(world, build_schedule(), command_rx);
        Self {
            command_tx,
            export_handle,
        }
    }

    // 获取当前会话状态
    pub fn get_game_session(&self) -> Session {
        let export_snapshot = self.export_handle.current_snapshot();
        Session::from_snapshot(export_snapshot)
    }

    // 订阅当前会话事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.export_handle.subscribe_events()
    }

    // 同步提交下一轮启动命令，实际推进由后台 runtime 处理。
    pub fn start_next_turn(&self) -> Result<(), String> {
        self.command_tx
            .send(EngineCommand::StartNextTurn)
            .map_err(|_| "会话运行时已停止，无法继续推进".to_string())
    }

    // 同步提交玩家选择，实际推进由后台 runtime 处理。
    pub fn submit_player_choice(&self, choice_id: &str) -> Result<(), String> {
        self.command_tx
            .send(EngineCommand::SubmitPlayerChoice {
                choice_id: choice_id.to_string(),
            })
            .map_err(|_| "会话运行时已停止，无法提交选择".to_string())
    }
}

impl Session {
    fn from_snapshot(snapshot: SessionSnapshot) -> Self {
        Self {
            phase: snapshot.phase,
            turn_index: snapshot.turn_index,
            active_turn_id: snapshot.active_turn_id,
            current_task: snapshot.current_task,
            tasks: snapshot.tasks,
            world_snapshot: snapshot.world,
            latest_narration: snapshot.latest_narration,
            current_protagonist_action: snapshot.current_protagonist_action,
            choices: snapshot.choices,
        }
    }
}

enum EngineCommand {
    StartNextTurn,
    SubmitPlayerChoice { choice_id: String },
}

struct SessionRuntime {
    world: World,
    schedule: Schedule,
    command_rx: mpsc::UnboundedReceiver<EngineCommand>,
}

impl SessionRuntime {
    fn spawn(world: World, schedule: Schedule, command_rx: mpsc::UnboundedReceiver<EngineCommand>) {
        tokio::spawn(async move {
            let mut runtime = Self {
                world,
                schedule,
                command_rx,
            };

            if let Err(err) = runtime.run_one_frame() {
                eprintln!("[akashic-session-runtime] 初始导出失败: {err}");
                return;
            }

            while let Some(command) = runtime.command_rx.recv().await {
                if let Err(err) = runtime.apply_command(command) {
                    eprintln!("[akashic-session-runtime] 命令执行失败: {err}");
                    break;
                }
                if let Err(err) = runtime.drive_until_stable().await {
                    eprintln!("[akashic-session-runtime] 会话推进失败: {err}");
                    break;
                }
            }
        });
    }

    async fn drive_until_stable(&mut self) -> Result<(), String> {
        loop {
            self.run_one_frame()?;
            let snapshot = Session::from_snapshot(self.export_snapshot());
            if is_stable_stop_point(&snapshot) {
                return Ok(());
            }
            sleep(DEFAULT_RUNTIME_TICK_INTERVAL).await;
        }
    }

    fn apply_command(&mut self, command: EngineCommand) -> Result<(), String> {
        match command {
            EngineCommand::StartNextTurn => {
                self.send_turn_control_command(TurnControl::StartNextTurn);
            }
            EngineCommand::SubmitPlayerChoice { choice_id } => {
                let turn_id = self.world.resource::<TurnState>().active_turn_id;
                self.world
                    .resource_mut::<PlayerInbox>()
                    .push(PlayerCommand::SubmitPlayerChoice { turn_id, choice_id });
            }
        }
        Ok(())
    }

    fn run_one_frame(&mut self) -> Result<(), String> {
        self.schedule.run(&mut self.world);
        self.ensure_not_failed()
    }

    fn ensure_not_failed(&self) -> Result<(), String> {
        let turn_state = self.world.resource::<TurnState>();
        if turn_state.phase == TurnPhase::Failed {
            return Err("游戏流程推进失败，请检查任务流或模型输出".to_string());
        }
        Ok(())
    }

    fn export_snapshot(&self) -> SessionSnapshot {
        self.world.resource::<ExportState>().current_snapshot()
    }

    fn send_turn_control_command(&mut self, control: TurnControl) {
        self.world
            .resource_mut::<Messages<TurnControl>>()
            .write(control);
    }
}

fn build_world(world_profile: &str, protagonist_profile: &str) -> (World, ExportHandle) {
    let mut world = World::new();

    world.init_resource::<WorldSnapshot>();
    world.init_resource::<TurnState>();
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::wait_for_user());
    world.init_resource::<ProtagonistDecisionState>();
    world.init_resource::<Messages<TurnEvent>>();
    world.init_resource::<Messages<TurnControl>>();
    world.insert_resource(TaskManager::new(build_chat_model()));
    let (export_state, export_handle) = ExportState::new_with_handle();
    world.insert_resource(export_state);

    world.spawn(FateWeaver::new(world_profile, protagonist_profile));
    world.spawn(UpperNarrator::new(world_profile, protagonist_profile));
    world.spawn(Protagonist::new(world_profile, protagonist_profile));

    (world, export_handle)
}

fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            task_system,
            message_update_system,
            player_input_system,
            turn_orchestrator_system,
            fate_weaver_system,
            fate_weaver_apply_system,
            upper_narrator_system,
            upper_narrator_apply_system,
            protagonist_system,
            protagonist_apply_system,
            export_system,
        )
            .chain(),
    );
    schedule
}

fn is_stable_stop_point(snapshot: &Session) -> bool {
    matches!(
        snapshot.phase,
        TurnPhase::AwaitingPlayerChoice | TurnPhase::TurnComplete
    ) || (snapshot.phase == TurnPhase::Idle && snapshot.current_task.is_none())
}
