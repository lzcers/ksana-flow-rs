use agent::agent::Context;
use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use serde::Serialize;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
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
        protagonist_action::{
            PendingProtagonistChoice, PlayerActionInput, ProtagonistDecisionState,
        },
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

#[derive(Debug, Clone)]
pub struct SessionArchiveState {
    pub world_profile: String,
    pub protagonist_profile: String,
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub world_snapshot: WorldSnapshot,
    pub committed_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
    pub fate_weaver_context: Context,
    pub upper_narrator_context: Context,
    pub protagonist_context: Context,
}

#[derive(Resource, Debug, Clone)]
struct SessionProfiles {
    world_profile: String,
    protagonist_profile: String,
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

    pub fn from_archive_state(state: SessionArchiveState) -> Result<Self, String> {
        validate_archive_state(&state)?;
        let (world, export_handle) = build_world_from_archive(state);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        SessionRuntime::spawn(world, build_schedule(), command_rx);
        Ok(Self {
            command_tx,
            export_handle,
        })
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

    // 同步提交玩家行动，实际推进由后台 runtime 处理。
    pub fn submit_player_action(&self, input: PlayerActionInput) -> Result<(), String> {
        self.command_tx
            .send(EngineCommand::SubmitPlayerAction { input })
            .map_err(|_| "会话运行时已停止，无法提交行动".to_string())
    }

    pub async fn export_archive_state(&self) -> Result<SessionArchiveState, String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(EngineCommand::ExportArchiveState { tx })
            .map_err(|_| "会话运行时已停止，无法导出存档".to_string())?;
        rx.await
            .map_err(|_| "会话运行时已停止，无法接收存档导出结果".to_string())?
    }

    pub async fn wait_until_ready(&self) -> Result<(), String> {
        let mut snapshot_rx = self.export_handle.snapshot_receiver();
        snapshot_rx
            .changed()
            .await
            .map_err(|_| "会话运行时已停止，无法等待初始化完成".to_string())
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
    SubmitPlayerAction {
        input: PlayerActionInput,
    },
    ExportArchiveState {
        tx: oneshot::Sender<Result<SessionArchiveState, String>>,
    },
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
                match command {
                    EngineCommand::ExportArchiveState { tx } => {
                        let _ = tx.send(runtime.export_archive_state());
                    }
                    EngineCommand::StartNextTurn => {
                        runtime.send_turn_control_command(TurnControl::StartNextTurn);
                        if let Err(err) = runtime.drive_until_stable().await {
                            eprintln!("[akashic-session-runtime] 会话推进失败: {err}");
                            break;
                        }
                    }
                    EngineCommand::SubmitPlayerAction { input } => {
                        let turn_id = runtime.world.resource::<TurnState>().active_turn_id;
                        runtime
                            .world
                            .resource_mut::<PlayerInbox>()
                            .push(PlayerCommand::SubmitPlayerAction { turn_id, input });
                        if let Err(err) = runtime.drive_until_stable().await {
                            eprintln!("[akashic-session-runtime] 会话推进失败: {err}");
                            break;
                        }
                    }
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

    fn export_archive_state(&mut self) -> Result<SessionArchiveState, String> {
        let world = &mut self.world;
        let snapshot = {
            let export_state = world.resource::<ExportState>();
            Session::from_snapshot(export_state.current_snapshot())
        };
        if !is_stable_stop_point(&snapshot) {
            return Err("当前会话不在稳定态，无法创建存档".to_string());
        }

        let profiles = world.resource::<SessionProfiles>().clone();
        let (phase, turn_index, active_turn_id) = {
            let turn_state = world.resource::<TurnState>();
            (
                turn_state.phase,
                turn_state.turn_index,
                turn_state.active_turn_id,
            )
        };
        let world_snapshot = world.resource::<WorldSnapshot>().clone();
        let (committed_action, choices) = {
            let protagonist_decision = world.resource::<ProtagonistDecisionState>();
            (
                protagonist_decision.committed_action().to_string(),
                protagonist_decision.choices().to_vec(),
            )
        };
        let fate_weaver_context = {
            let mut query = world.query::<&FateWeaver>();
            query
                .single(world)
                .map_err(|_| "未找到 FateWeaver 实体，无法导出存档".to_string())?
                .context()
                .clone()
        };
        let upper_narrator_context = {
            let mut query = world.query::<&UpperNarrator>();
            query
                .single(world)
                .map_err(|_| "未找到 UpperNarrator 实体，无法导出存档".to_string())?
                .context()
                .clone()
        };
        let protagonist_context = {
            let mut query = world.query::<&Protagonist>();
            query
                .single(world)
                .map_err(|_| "未找到 Protagonist 实体，无法导出存档".to_string())?
                .context()
                .clone()
        };

        Ok(SessionArchiveState {
            world_profile: profiles.world_profile,
            protagonist_profile: profiles.protagonist_profile,
            phase,
            turn_index,
            active_turn_id,
            world_snapshot,
            committed_action,
            choices,
            fate_weaver_context,
            upper_narrator_context,
            protagonist_context,
        })
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
    world.insert_resource(SessionProfiles {
        world_profile: world_profile.to_string(),
        protagonist_profile: protagonist_profile.to_string(),
    });
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

fn build_world_from_archive(state: SessionArchiveState) -> (World, ExportHandle) {
    let mut world = World::new();

    world.insert_resource(state.world_snapshot);
    world.insert_resource(TurnState {
        phase: state.phase,
        turn_index: state.turn_index,
        active_turn_id: state.active_turn_id,
    });
    world.insert_resource(SessionProfiles {
        world_profile: state.world_profile,
        protagonist_profile: state.protagonist_profile,
    });
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::wait_for_user());
    world.insert_resource(ProtagonistDecisionState::from_archive(
        state.committed_action,
        state.choices,
    ));
    world.init_resource::<Messages<TurnEvent>>();
    world.init_resource::<Messages<TurnControl>>();
    world.insert_resource(TaskManager::new(build_chat_model()));
    let (export_state, export_handle) = ExportState::new_with_handle();
    world.insert_resource(export_state);

    world.spawn(FateWeaver::from_context(state.fate_weaver_context));
    world.spawn(UpperNarrator::from_context(state.upper_narrator_context));
    world.spawn(Protagonist::from_context(state.protagonist_context));

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

fn validate_archive_state(state: &SessionArchiveState) -> Result<(), String> {
    let stable_phase = matches!(
        state.phase,
        TurnPhase::Idle | TurnPhase::AwaitingPlayerChoice | TurnPhase::TurnComplete
    );
    if !stable_phase {
        return Err("归档会话不在可恢复的稳定态".to_string());
    }

    if state.active_turn_id < state.turn_index {
        return Err("归档会话的 active_turn_id 不能小于 turn_index".to_string());
    }

    Ok(())
}
