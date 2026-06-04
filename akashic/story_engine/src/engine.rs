use agent::agent::Context;
use bevy_ecs::{prelude::*, schedule::Schedule};
use serde::Serialize;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::{sleep, Duration},
};

use crate::{
    components::{
        agent::{Agent, AgentKind, FlowOwner},
        turn_flow::TurnFlow,
    },
    profile::{DEFAULT_KEY_STORY_BEATS, DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        export::{ExportHandle, ExportState, SessionSnapshot, TaskEvent, TaskView},
        history::{RoundHistoryEntry, SessionHistoryLog},
        llm_task_manager::TaskManager,
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PendingProtagonistChoice, PlayerActionInput, ProtagonistDecisionState},
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    },
    systems::{
        cleanup_sys::cleanup_previous_turn_outcomes_system,
        export_sys::export_system,
        fate_sys::{fate_apply_system, fate_dispatch_system},
        history_sys::history_sys,
        narration_sys::{narration_apply_system, narration_dispatch_system},
        player_input_sys::player_input_system,
        protagonist_sys::{protagonist_apply_system, protagonist_dispatch_system},
        scheduler::agent_scheduler_system,
        task_sys::task_poll_system,
    },
    turn_messages::PlayerCommand,
    utils::build_chat_model,
};

const DEFAULT_RUNTIME_TICK_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub history: Vec<RoundHistoryEntry>,
    pub current_task: Option<TaskView>,
    pub tasks: Vec<TaskView>,
    pub world_snapshot: WorldSnapshot,
    pub latest_narration: String,
    pub current_protagonist_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
}

#[derive(Debug, Clone)]
pub struct SessionArchiveState {
    pub world_profile: String,
    pub protagonist_profile: String,
    pub key_story_beats: String,
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub world_snapshot: WorldSnapshot,
    pub committed_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
    pub history_log: SessionHistoryLog,
    pub fate_weaver_context: Context,
    pub upper_narrator_context: Context,
    pub protagonist_context: Context,
}

#[derive(Resource, Debug, Clone)]
struct SessionProfiles {
    world_profile: String,
    protagonist_profile: String,
    key_story_beats: String,
}

pub struct AkashicSessionEngine {
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    export_handle: ExportHandle,
}

impl AkashicSessionEngine {
    pub fn new() -> Self {
        Self::new_with_profiles(
            DEFAULT_WORLD_PROFILE,
            DEFAULT_PROTAGONIST_PROFILE,
            DEFAULT_KEY_STORY_BEATS,
        )
    }

    pub fn new_with_profiles(
        world_profile: &str,
        protagonist_profile: &str,
        key_story_beats: &str,
    ) -> Self {
        let (world, export_handle) = build_world(world_profile, protagonist_profile, key_story_beats);
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

    pub fn get_game_session(&self) -> Session {
        Session::from_snapshot(self.export_handle.current_snapshot())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.export_handle.subscribe_events()
    }

    pub fn start_next_turn(&self) -> Result<(), String> {
        if self.get_game_session().phase == TurnPhase::StoryEnded {
            return Err("故事已结束，无法继续推进".to_string());
        }
        self.command_tx
            .send(EngineCommand::StartNextTurn)
            .map_err(|_| "会话运行时已停止，无法继续推进".to_string())
    }

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
            history: snapshot.history,
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
    SubmitPlayerAction { input: PlayerActionInput },
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
                eprintln!("[story-engine-runtime] 初始导出失败: {err}");
                return;
            }

            while let Some(command) = runtime.command_rx.recv().await {
                match command {
                    EngineCommand::ExportArchiveState { tx } => {
                        let _ = tx.send(runtime.export_archive_state());
                    }
                    EngineCommand::StartNextTurn => {
                        runtime.advance_turn();
                        if let Err(err) = runtime.drive_until_stable().await {
                            eprintln!("[story-engine-runtime] 会话推进失败: {err}");
                            break;
                        }
                    }
                    EngineCommand::SubmitPlayerAction { input } => {
                        let turn_id = runtime.current_flow().map(|flow| flow.active_turn_id).unwrap_or_default();
                        runtime
                            .world
                            .resource_mut::<PlayerInbox>()
                            .push(PlayerCommand::SubmitPlayerAction { turn_id, input });
                        if let Err(err) = runtime.drive_until_stable().await {
                            eprintln!("[story-engine-runtime] 会话推进失败: {err}");
                            break;
                        }
                    }
                }
            }
        });
    }

    fn advance_turn(&mut self) {
        let mut query = self.world.query::<&mut TurnFlow>();
        if let Ok(mut flow) = query.single_mut(&mut self.world) {
            flow.advance();
        }
    }

    fn current_flow(&mut self) -> Option<TurnFlow> {
        let mut query = self.world.query::<&TurnFlow>();
        query.single(&self.world).ok().copied()
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

    fn export_snapshot(&self) -> SessionSnapshot {
        self.world.resource::<ExportState>().current_snapshot()
    }

    fn ensure_not_failed(&mut self) -> Result<(), String> {
        let mut query = self.world.query::<&TurnFlow>();
        let Ok(flow) = query.single(&self.world) else {
            return Ok(());
        };
        if flow.stage == TurnPhase::Failed {
            return Err("会话运行失败".to_string());
        }
        Ok(())
    }

    fn export_archive_state(&mut self) -> Result<SessionArchiveState, String> {
        let snapshot = {
            let export_state = self.world.resource::<ExportState>();
            Session::from_snapshot(export_state.current_snapshot())
        };
        if !is_stable_stop_point(&snapshot) {
            return Err("当前会话不在稳定态，无法创建存档".to_string());
        }

        let profiles = self.world.resource::<SessionProfiles>().clone();
        let world_snapshot = self.world.resource::<WorldSnapshot>().clone();
        let history_log = self.world.resource::<SessionHistoryLog>().clone();
        let decision_state = self.world.resource::<ProtagonistDecisionState>().clone();
        let flow = self.current_flow().ok_or_else(|| "缺少流程实体".to_string())?;

        let mut fate_weaver_context = None;
        let mut upper_narrator_context = None;
        let mut protagonist_context = None;
        let mut query = self.world.query::<&Agent>();
        for agent in query.iter(&self.world) {
            match agent.kind {
                AgentKind::FateWeaver => fate_weaver_context = Some(agent.context.clone()),
                AgentKind::UpperNarrator => upper_narrator_context = Some(agent.context.clone()),
                AgentKind::Protagonist => protagonist_context = Some(agent.context.clone()),
            }
        }

        Ok(SessionArchiveState {
            world_profile: profiles.world_profile,
            protagonist_profile: profiles.protagonist_profile,
            key_story_beats: profiles.key_story_beats,
            phase: flow.stage,
            turn_index: flow.turn_index,
            active_turn_id: flow.active_turn_id,
            world_snapshot,
            committed_action: decision_state.committed_action().to_string(),
            choices: decision_state.choices().to_vec(),
            history_log,
            fate_weaver_context: fate_weaver_context
                .ok_or_else(|| "缺少 FateWeaver 上下文".to_string())?,
            upper_narrator_context: upper_narrator_context
                .ok_or_else(|| "缺少 UpperNarrator 上下文".to_string())?,
            protagonist_context: protagonist_context
                .ok_or_else(|| "缺少 Protagonist 上下文".to_string())?,
        })
    }
}

fn build_world(
    world_profile: &str,
    protagonist_profile: &str,
    key_story_beats: &str,
) -> (World, ExportHandle) {
    let mut world = World::new();

    world.init_resource::<WorldSnapshot>();
    world.init_resource::<SessionHistoryLog>();
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::wait_for_user());
    world.init_resource::<ProtagonistDecisionState>();
    world.insert_resource(SessionProfiles {
        world_profile: world_profile.to_string(),
        protagonist_profile: protagonist_profile.to_string(),
        key_story_beats: key_story_beats.to_string(),
    });
    world.insert_resource(TaskManager::new(build_chat_model()));
    let (export_state, export_handle) = ExportState::new_with_handle();
    world.insert_resource(export_state);

    let flow_entity = world.spawn(TurnFlow::default()).id();
    world.spawn((
        Agent::new_fate_weaver(world_profile, protagonist_profile, key_story_beats),
        FlowOwner(flow_entity),
    ));
    world.spawn((
        Agent::new_upper_narrator(world_profile, protagonist_profile),
        FlowOwner(flow_entity),
    ));
    world.spawn((
        Agent::new_protagonist(world_profile, protagonist_profile),
        FlowOwner(flow_entity),
    ));

    (world, export_handle)
}

fn build_world_from_archive(state: SessionArchiveState) -> (World, ExportHandle) {
    let mut world = World::new();

    world.insert_resource(state.world_snapshot);
    world.insert_resource(state.history_log);
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::wait_for_user());
    world.insert_resource(ProtagonistDecisionState::from_archive(
        state.committed_action,
        state.choices,
    ));
    world.insert_resource(SessionProfiles {
        world_profile: state.world_profile,
        protagonist_profile: state.protagonist_profile,
        key_story_beats: state.key_story_beats,
    });
    world.insert_resource(TaskManager::new(build_chat_model()));
    let (export_state, export_handle) = ExportState::new_with_handle();
    world.insert_resource(export_state);

    let flow_entity = world
        .spawn(TurnFlow {
            turn_index: state.turn_index,
            active_turn_id: state.active_turn_id,
            stage: state.phase,
        })
        .id();
    world.spawn((
        Agent::from_context(
            AgentKind::FateWeaver,
            "FateWeaver",
            String::new(),
            state.fate_weaver_context,
        ),
        FlowOwner(flow_entity),
    ));
    world.spawn((
        Agent::from_context(
            AgentKind::UpperNarrator,
            "UpperNarrator",
            String::new(),
            state.upper_narrator_context,
        ),
        FlowOwner(flow_entity),
    ));
    world.spawn((
        Agent::from_context(
            AgentKind::Protagonist,
            "Protagonist",
            String::new(),
            state.protagonist_context,
        ),
        FlowOwner(flow_entity),
    ));

    (world, export_handle)
}

fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            (
                cleanup_previous_turn_outcomes_system,
                fate_dispatch_system,
                narration_dispatch_system,
                protagonist_dispatch_system,
            )
                .chain(),
            (
                agent_scheduler_system,
                task_poll_system,
                fate_apply_system,
                narration_apply_system,
            )
                .chain(),
            (protagonist_apply_system, player_input_system, history_sys, export_system).chain(),
        )
            .chain(),
    );
    schedule
}

fn is_stable_stop_point(snapshot: &Session) -> bool {
    matches!(
        snapshot.phase,
        TurnPhase::AwaitingPlayerChoice | TurnPhase::TurnComplete | TurnPhase::StoryEnded
    ) || (snapshot.phase == TurnPhase::Idle && snapshot.current_task.is_none())
}

fn validate_archive_state(state: &SessionArchiveState) -> Result<(), String> {
    let stable_phase = matches!(
        state.phase,
        TurnPhase::Idle
            | TurnPhase::AwaitingPlayerChoice
            | TurnPhase::TurnComplete
            | TurnPhase::StoryEnded
    );
    if !stable_phase {
        return Err("归档会话不在可恢复的稳定态".to_string());
    }

    if state.active_turn_id < state.turn_index {
        return Err("归档会话的 active_turn_id 不能小于 turn_index".to_string());
    }

    Ok(())
}
