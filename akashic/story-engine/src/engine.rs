use std::collections::HashMap;

use agent::{agent::Context, models::ChatModel};
use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::{Schedule, SystemSet},
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::{Duration, MissedTickBehavior, interval},
};

use crate::{
    components::{
        agent::{Agent, AgentOutputType, Applicator, Simulator},
        session::{SessionProfiles, StorySession},
        turn_flow::{TurnFlow, TurnStage},
    },
    profile::{DEFAULT_KEY_STORY_BEATS, DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        agent_task::AgentTaskManager,
        export::{ExportHandle, ExportState, SessionSnapshot, TaskEvent, TaskView},
        history::{RoundHistoryEntry, SessionHistoryLog},
        player_input::PlayerInputConfig,
        protagonist_action::{
            PendingProtagonistChoice, PlayerActionInput, ProtagonistDecisionState,
        },
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    },
    systems::{
        agents::{
            fate_weaver_sys::{fate_weaver_apply_system, fate_weaver_dispatch_system},
            narration_sys::{narration_apply_system, narration_dispatch_system},
            player_sys::player_input_consume_system,
            protagonist_sys::{protagonist_apply_system, protagonist_dispatch_system},
        },
        export_sys::export_system,
        flow::{agent_task_system, cleanup_previous_turn_outcomes_system, flow_progress_system},
        history_sys::history_sys,
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
    pub simulators: Vec<SimulatorArchiveState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentArchiveKind {
    Simulator,
    Applicator,
    Player,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorArchiveState {
    #[serde(default = "default_simulator_kind")]
    pub kind: AgentArchiveKind,
    #[serde(alias = "output_kind")]
    pub output_type: AgentOutputType,
    pub name: String,
    #[serde(default)]
    pub sys_prompt: String,
    pub context: Context,
}

fn default_simulator_kind() -> AgentArchiveKind {
    AgentArchiveKind::Simulator
}

#[derive(Resource, Default)]
struct SessionRegistry {
    entities: HashMap<String, Entity>,
}

#[derive(Clone)]
pub struct AkashicEngine {
    command_tx: mpsc::UnboundedSender<EngineCommand>,
}

#[derive(Clone)]
pub struct AkashicSessionEngine {
    session_id: String,
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    export_handle: ExportHandle,
}

impl Default for AkashicEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AkashicEngine {
    pub fn new() -> Self {
        Self::with_model(build_chat_model())
    }

    pub fn with_model(model: ChatModel) -> Self {
        let mut world = World::new();
        world.insert_resource(AgentTaskManager::new(model));
        world.insert_resource(Messages::<PlayerCommand>::default());
        world.init_resource::<SessionRegistry>();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        SessionRuntime::spawn(world, build_schedule(), command_tx.clone(), command_rx);
        Self { command_tx }
    }

    pub async fn create_session(
        &self,
        session_id: impl Into<String>,
        world_profile: &str,
        protagonist_profile: &str,
        key_story_beats: &str,
    ) -> Result<AkashicSessionEngine, String> {
        self.create_session_from_state(
            session_id.into(),
            NewSessionState::Profiles {
                world_profile: world_profile.to_string(),
                protagonist_profile: protagonist_profile.to_string(),
                key_story_beats: key_story_beats.to_string(),
            },
        )
        .await
    }

    pub async fn create_default_session(
        &self,
        session_id: impl Into<String>,
    ) -> Result<AkashicSessionEngine, String> {
        self.create_session(
            session_id,
            DEFAULT_WORLD_PROFILE,
            DEFAULT_PROTAGONIST_PROFILE,
            DEFAULT_KEY_STORY_BEATS,
        )
        .await
    }

    pub async fn create_session_from_archive(
        &self,
        session_id: impl Into<String>,
        state: SessionArchiveState,
    ) -> Result<AkashicSessionEngine, String> {
        validate_archive_state(&state)?;
        self.create_session_from_state(session_id.into(), NewSessionState::Archive(Box::new(state)))
            .await
    }

    async fn create_session_from_state(
        &self,
        session_id: String,
        state: NewSessionState,
    ) -> Result<AkashicSessionEngine, String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(EngineCommand::CreateSession {
                session_id,
                state,
                tx,
            })
            .map_err(|_| "故事引擎运行时已停止，无法创建会话".to_string())?;
        rx.await
            .map_err(|_| "故事引擎运行时已停止，无法接收会话创建结果".to_string())?
    }
}

impl AkashicSessionEngine {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn get_game_session(&self) -> Session {
        Session::from_snapshot(self.export_handle.current_snapshot())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.export_handle.subscribe_events()
    }

    pub fn start_next_turn(&self) -> Result<(), String> {
        if self.get_game_session().phase == TurnPhase::Ended {
            return Err("故事已结束，无法继续推进".to_string());
        }
        self.command_tx
            .send(EngineCommand::StartNextTurn {
                session_id: self.session_id.clone(),
            })
            .map_err(|_| "故事引擎运行时已停止，无法继续推进".to_string())
    }

    pub fn submit_player_action(&self, input: PlayerActionInput) -> Result<(), String> {
        self.command_tx
            .send(EngineCommand::SubmitPlayerAction {
                session_id: self.session_id.clone(),
                input,
            })
            .map_err(|_| "故事引擎运行时已停止，无法提交行动".to_string())
    }

    pub async fn export_archive_state(&self) -> Result<SessionArchiveState, String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(EngineCommand::ExportArchiveState {
                session_id: self.session_id.clone(),
                tx,
            })
            .map_err(|_| "故事引擎运行时已停止，无法导出存档".to_string())?;
        rx.await
            .map_err(|_| "故事引擎运行时已停止，无法接收存档导出结果".to_string())?
    }

    pub async fn add_simulator(&self, simulator: Agent) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(EngineCommand::AddSimulator {
                session_id: self.session_id.clone(),
                simulator,
                tx,
            })
            .map_err(|_| "故事引擎运行时已停止，无法添加 Simulator".to_string())?;
        rx.await
            .map_err(|_| "故事引擎运行时已停止，无法接收 Simulator 添加结果".to_string())?
    }

    pub async fn wait_until_ready(&self) -> Result<(), String> {
        Ok(())
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

enum NewSessionState {
    Profiles {
        world_profile: String,
        protagonist_profile: String,
        key_story_beats: String,
    },
    Archive(Box<SessionArchiveState>),
}

enum EngineCommand {
    CreateSession {
        session_id: String,
        state: NewSessionState,
        tx: oneshot::Sender<Result<AkashicSessionEngine, String>>,
    },
    StartNextTurn {
        session_id: String,
    },
    SubmitPlayerAction {
        session_id: String,
        input: PlayerActionInput,
    },
    AddSimulator {
        session_id: String,
        simulator: Agent,
        tx: oneshot::Sender<Result<(), String>>,
    },
    ExportArchiveState {
        session_id: String,
        tx: oneshot::Sender<Result<SessionArchiveState, String>>,
    },
}

struct SessionRuntime {
    world: World,
    schedule: Schedule,
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    command_rx: mpsc::UnboundedReceiver<EngineCommand>,
}

impl SessionRuntime {
    fn spawn(
        world: World,
        schedule: Schedule,
        command_tx: mpsc::UnboundedSender<EngineCommand>,
        command_rx: mpsc::UnboundedReceiver<EngineCommand>,
    ) {
        tokio::spawn(async move {
            let mut runtime = Self {
                world,
                schedule,
                command_tx,
                command_rx,
            };
            let mut ticker = interval(DEFAULT_RUNTIME_TICK_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    command = runtime.command_rx.recv() => {
                        let Some(command) = command else {
                            break;
                        };
                        runtime.handle_command(command);
                    }
                    _ = ticker.tick(), if runtime.has_active_work() => {
                        runtime.run_one_frame();
                    }
                }
            }
        });
    }

    fn handle_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::CreateSession {
                session_id,
                state,
                tx,
            } => {
                let result = self.create_session(session_id, state);
                self.run_one_frame();
                let _ = tx.send(result);
            }
            EngineCommand::StartNextTurn { session_id } => {
                if let Some(entity) = self.session_entity(&session_id)
                    && let Some(mut flow) = self.world.get_mut::<TurnFlow>(entity)
                {
                    flow.advance();
                    self.run_one_frame();
                }
            }
            EngineCommand::SubmitPlayerAction { session_id, input } => {
                if let Some(entity) = self.session_entity(&session_id) {
                    let turn_id = self
                        .world
                        .get::<TurnFlow>(entity)
                        .map(|flow| flow.active_turn_id)
                        .unwrap_or_default();
                    self.world.resource_mut::<Messages<PlayerCommand>>().write(
                        PlayerCommand::SubmitPlayerAction {
                            session_entity: entity,
                            turn_id,
                            input,
                        },
                    );
                    self.run_one_frame();
                }
            }
            EngineCommand::AddSimulator {
                session_id,
                simulator,
                tx,
            } => {
                let _ = tx.send(self.add_simulator(&session_id, simulator));
            }
            EngineCommand::ExportArchiveState { session_id, tx } => {
                let _ = tx.send(self.export_archive_state(&session_id));
            }
        }
    }

    fn create_session(
        &mut self,
        session_id: String,
        state: NewSessionState,
    ) -> Result<AkashicSessionEngine, String> {
        self.remove_session(&session_id);

        let (export_state, export_handle) = ExportState::new_with_handle();
        let session_entity = match state {
            NewSessionState::Profiles {
                world_profile,
                protagonist_profile,
                key_story_beats,
            } => self.spawn_session_from_profiles(
                &session_id,
                world_profile,
                protagonist_profile,
                key_story_beats,
                export_state,
            ),
            NewSessionState::Archive(state) => {
                self.spawn_session_from_archive(&session_id, *state, export_state)
            }
        }?;
        self.world
            .resource_mut::<SessionRegistry>()
            .entities
            .insert(session_id.clone(), session_entity);

        Ok(AkashicSessionEngine {
            session_id,
            command_tx: self.command_tx.clone(),
            export_handle,
        })
    }

    fn remove_session(&mut self, session_id: &str) {
        let Some(session_entity) = self
            .world
            .resource_mut::<SessionRegistry>()
            .entities
            .remove(session_id)
        else {
            return;
        };
        for agent_entity in self.owned_agent_entities(session_entity) {
            self.world
                .resource_mut::<AgentTaskManager>()
                .clear_task(agent_entity);
            self.world.despawn(agent_entity);
        }
        self.world.despawn(session_entity);
    }

    fn add_simulator(&mut self, session_id: &str, simulator: Agent) -> Result<(), String> {
        let session_entity = self
            .session_entity(session_id)
            .ok_or_else(|| format!("未找到会话 `{session_id}`"))?;
        let flow = self
            .world
            .get::<TurnFlow>(session_entity)
            .ok_or_else(|| "会话缺少流程状态".to_string())?;
        if !is_stable_phase(flow.stage) {
            return Err("只能在会话稳定阶段添加 Agent".to_string());
        }
        if simulator.output_type != AgentOutputType::Json {
            return Err("动态 Simulator 只支持 JSON 输出".to_string());
        }
        if simulator.output_type == AgentOutputType::Json && self.has_json_simulator(session_entity)
        {
            return Err("每个会话只能存在一个 JSON Simulator".to_string());
        }
        self.world
            .spawn((simulator, ChildOf(session_entity), Simulator));
        Ok(())
    }

    fn spawn_session_from_profiles(
        &mut self,
        session_id: &str,
        world_profile: String,
        protagonist_profile: String,
        key_story_beats: String,
        export_state: ExportState,
    ) -> Result<Entity, String> {
        let session_entity = self
            .world
            .spawn((
                StorySession {
                    id: session_id.to_string(),
                },
                SessionProfiles {
                    world_profile: world_profile.clone(),
                    protagonist_profile: protagonist_profile.clone(),
                    key_story_beats: key_story_beats.clone(),
                },
                TurnFlow::default(),
                WorldSnapshot::default(),
                SessionHistoryLog::default(),
                PlayerInputConfig::wait_for_user(),
                ProtagonistDecisionState::default(),
                export_state,
            ))
            .id();
        self.spawn_agents(
            session_entity,
            vec![Agent::new_fate_weaver(
                &world_profile,
                &protagonist_profile,
                &key_story_beats,
            )],
            Agent::new_upper_narrator(&world_profile, &protagonist_profile),
            Agent::new_protagonist(&world_profile, &protagonist_profile),
        );
        Ok(session_entity)
    }

    fn spawn_session_from_archive(
        &mut self,
        session_id: &str,
        state: SessionArchiveState,
        export_state: ExportState,
    ) -> Result<Entity, String> {
        let session_entity = self
            .world
            .spawn((
                StorySession {
                    id: session_id.to_string(),
                },
                SessionProfiles {
                    world_profile: state.world_profile,
                    protagonist_profile: state.protagonist_profile,
                    key_story_beats: state.key_story_beats,
                },
                TurnFlow {
                    turn_index: state.turn_index,
                    active_turn_id: state.active_turn_id,
                    stage: state.phase,
                },
                state.world_snapshot,
                state.history_log,
                PlayerInputConfig::wait_for_user(),
                ProtagonistDecisionState::from_archive(state.committed_action, state.choices),
                export_state,
            ))
            .id();
        let simulators = if state.simulators.is_empty() {
            vec![Agent::from_context(
                AgentOutputType::Json,
                "FateWeaver",
                "",
                state.fate_weaver_context,
            )]
        } else {
            state
                .simulators
                .into_iter()
                .map(|simulator| {
                    Agent::from_context(
                        simulator.output_type,
                        simulator.name,
                        simulator.sys_prompt,
                        simulator.context,
                    )
                })
                .collect()
        };
        self.spawn_agents(
            session_entity,
            simulators,
            Agent::from_context(
                AgentOutputType::Text,
                "UpperNarrator",
                "",
                state.upper_narrator_context,
            ),
            Agent::from_context(
                AgentOutputType::Json,
                "Protagonist",
                "",
                state.protagonist_context,
            ),
        );
        Ok(session_entity)
    }

    fn spawn_agents(
        &mut self,
        session_entity: Entity,
        simulators: Vec<Agent>,
        narrator: Agent,
        protagonist: Agent,
    ) {
        for agent in simulators {
            self.world
                .spawn((agent, ChildOf(session_entity), Simulator));
        }
        self.world
            .spawn((narrator, ChildOf(session_entity), Applicator));
        self.world
            .spawn((protagonist, ChildOf(session_entity), Applicator));
    }

    fn session_entity(&self, session_id: &str) -> Option<Entity> {
        self.world
            .resource::<SessionRegistry>()
            .entities
            .get(session_id)
            .copied()
    }

    fn run_one_frame(&mut self) {
        self.schedule.run(&mut self.world);
    }

    fn has_active_work(&mut self) -> bool {
        let mut query = self.world.query::<&TurnFlow>();
        query
            .iter(&self.world)
            .any(|flow| !is_stable_phase(flow.stage))
    }

    fn export_archive_state(&mut self, session_id: &str) -> Result<SessionArchiveState, String> {
        let entity = self
            .session_entity(session_id)
            .ok_or_else(|| format!("未找到会话 `{session_id}`"))?;
        let flow = *self
            .world
            .get::<TurnFlow>(entity)
            .ok_or_else(|| "会话缺少流程状态".to_string())?;
        if !is_stable_phase(flow.stage) {
            return Err("当前会话不在稳定态，无法创建存档".to_string());
        }
        let profiles = self
            .world
            .get::<SessionProfiles>(entity)
            .ok_or_else(|| "会话缺少配置资料".to_string())?
            .clone();
        let world_snapshot = self
            .world
            .get::<WorldSnapshot>(entity)
            .ok_or_else(|| "会话缺少世界快照".to_string())?
            .clone();
        let history_log = self
            .world
            .get::<SessionHistoryLog>(entity)
            .ok_or_else(|| "会话缺少历史记录".to_string())?
            .clone();
        let decision_state = self
            .world
            .get::<ProtagonistDecisionState>(entity)
            .ok_or_else(|| "会话缺少主角决策状态".to_string())?
            .clone();
        let simulator_agents = self.owned_simulator_agents(entity);

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
            fate_weaver_context: simulator_agents
                .iter()
                .find(|agent| agent.output_type == AgentOutputType::Json)
                .map(|agent| agent.context.clone())
                .ok_or_else(|| "缺少 JSON Simulator".to_string())?,
            upper_narrator_context: self
                .unique_applicator_context(entity, AgentOutputType::Text)?,
            protagonist_context: self.unique_applicator_context(entity, AgentOutputType::Json)?,
            simulators: simulator_agents
                .iter()
                .map(|agent| SimulatorArchiveState {
                    kind: AgentArchiveKind::Simulator,
                    output_type: agent.output_type,
                    name: agent.name.clone(),
                    sys_prompt: agent.sys_prompt.clone(),
                    context: agent.context.clone(),
                })
                .collect(),
        })
    }

    fn owned_agent_entities(&mut self, session_entity: Entity) -> Vec<Entity> {
        let mut agents = self.world.query::<(Entity, &ChildOf)>();
        agents
            .iter(&self.world)
            .filter_map(|(entity, owner)| (owner.parent() == session_entity).then_some(entity))
            .collect()
    }

    fn owned_simulator_agents(&mut self, session_entity: Entity) -> Vec<Agent> {
        let mut agents = self.world.query::<(&Agent, &ChildOf, &Simulator)>();
        agents
            .iter(&self.world)
            .filter(move |(_, owner, _)| owner.parent() == session_entity)
            .map(|(agent, _, _)| agent.clone())
            .collect()
    }

    fn unique_applicator_context(
        &mut self,
        session_entity: Entity,
        output_type: AgentOutputType,
    ) -> Result<Context, String> {
        let mut agents = self.world.query::<(&Agent, &ChildOf, &Applicator)>();
        let mut matches = agents.iter(&self.world).filter(|(agent, owner, _)| {
            owner.parent() == session_entity && agent.output_type == output_type
        });
        let context = matches
            .next()
            .map(|(agent, ..)| agent.context.clone())
            .ok_or_else(|| format!("缺少 Applicator/{output_type:?} Agent"))?;
        if matches.next().is_some() {
            return Err(format!("存在多个 Applicator/{output_type:?} Agent"));
        }
        Ok(context)
    }

    fn has_json_simulator(&mut self, session_entity: Entity) -> bool {
        let mut agents = self.world.query::<(&Agent, &ChildOf, &Simulator)>();
        agents.iter(&self.world).any(|(agent, owner, _)| {
            owner.parent() == session_entity && agent.output_type == AgentOutputType::Json
        })
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum StoryScheduleSet {
    Dispatch,
    PollTasks,
    ApplyResults,
    Progress,
    Finalize,
}

fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.configure_sets(
        (
            StoryScheduleSet::Dispatch,
            StoryScheduleSet::PollTasks,
            StoryScheduleSet::ApplyResults,
            StoryScheduleSet::Progress,
            StoryScheduleSet::Finalize,
        )
            .chain(),
    );
    schedule.add_systems(
        (
            fate_weaver_dispatch_system,
            narration_dispatch_system,
            protagonist_dispatch_system,
        )
            .in_set(StoryScheduleSet::Dispatch),
    );
    schedule.add_systems(agent_task_system.in_set(StoryScheduleSet::PollTasks));
    schedule.add_systems(
        (
            fate_weaver_apply_system,
            narration_apply_system,
            protagonist_apply_system,
            player_input_consume_system,
        )
            .in_set(StoryScheduleSet::ApplyResults),
    );
    schedule.add_systems(flow_progress_system.in_set(StoryScheduleSet::Progress));
    schedule.add_systems(
        (
            history_sys,
            export_system,
            cleanup_previous_turn_outcomes_system,
            message_update_system,
        )
            .chain()
            .in_set(StoryScheduleSet::Finalize),
    );
    schedule
}

fn is_stable_phase(phase: TurnStage) -> bool {
    matches!(
        phase,
        TurnStage::Idle
            | TurnStage::AwaitingPlayer
            | TurnStage::TurnCompleted
            | TurnStage::Ended
            | TurnStage::Failed
    )
}

fn validate_archive_state(state: &SessionArchiveState) -> Result<(), String> {
    if !is_stable_phase(state.phase) || state.phase == TurnStage::Failed {
        return Err("归档会话不在可恢复的稳定态".to_string());
    }
    if state.active_turn_id < state.turn_index {
        return Err("归档会话的 active_turn_id 不能小于 turn_index".to_string());
    }
    if state
        .simulators
        .iter()
        .any(|simulator| simulator.kind != AgentArchiveKind::Simulator)
    {
        return Err("归档的 simulators 只能包含 Simulator".to_string());
    }
    if !state.simulators.is_empty()
        && state
            .simulators
            .iter()
            .filter(|simulator| simulator.output_type == AgentOutputType::Json)
            .count()
            != 1
    {
        return Err("归档必须包含且只能包含一个 JSON Simulator".to_string());
    }
    if state
        .simulators
        .iter()
        .any(|simulator| simulator.output_type != AgentOutputType::Json)
    {
        return Err("归档的 simulators 只能包含 JSON Simulator".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn keeps_profiles_isolated_across_sessions_in_one_world() {
        let engine = AkashicEngine::with_model(ChatModel::new());
        let first = engine
            .create_session("first", "world-a", "hero-a", "beats-a")
            .await
            .unwrap();
        let second = engine
            .create_session("second", "world-b", "hero-b", "beats-b")
            .await
            .unwrap();

        let first_archive = first.export_archive_state().await.unwrap();
        let second_archive = second.export_archive_state().await.unwrap();

        assert_eq!(first_archive.world_profile, "world-a");
        assert_eq!(second_archive.world_profile, "world-b");
        assert_eq!(first_archive.simulators.len(), 1);
        assert_eq!(second_archive.simulators.len(), 1);
        assert_eq!(first.session_id(), "first");
        assert_eq!(second.session_id(), "second");
    }

    #[tokio::test]
    async fn rejects_dynamically_added_text_simulators() {
        let engine = AkashicEngine::with_model(ChatModel::new());
        let session = engine.create_default_session("session").await.unwrap();
        let result = session
            .add_simulator(Agent::new(
                AgentOutputType::Text,
                "WeatherSimulator",
                "simulate weather".to_string(),
            ))
            .await;

        assert_eq!(result.err().unwrap(), "动态 Simulator 只支持 JSON 输出");
    }

    #[tokio::test]
    async fn archive_rejects_text_simulators() {
        let engine = AkashicEngine::with_model(ChatModel::new());
        let session = engine.create_default_session("session").await.unwrap();
        let mut archive = session.export_archive_state().await.unwrap();
        archive.simulators.push(SimulatorArchiveState {
            kind: AgentArchiveKind::Simulator,
            output_type: AgentOutputType::Text,
            name: "WeatherSimulator".to_string(),
            sys_prompt: String::new(),
            context: Context::default(),
        });

        let result = engine
            .create_session_from_archive("restored", archive)
            .await;

        assert_eq!(
            result.err().unwrap(),
            "归档的 simulators 只能包含 JSON Simulator"
        );
    }

    #[tokio::test]
    async fn archive_rejects_narrator_in_simulators() {
        let engine = AkashicEngine::with_model(ChatModel::new());
        let session = engine.create_default_session("session").await.unwrap();
        let mut archive = session.export_archive_state().await.unwrap();
        archive.simulators.push(SimulatorArchiveState {
            kind: AgentArchiveKind::Applicator,
            output_type: AgentOutputType::Text,
            name: "AnotherNarrator".to_string(),
            sys_prompt: String::new(),
            context: Context::default(),
        });

        let result = engine
            .create_session_from_archive("restored", archive)
            .await;

        assert_eq!(
            result.err().unwrap(),
            "归档的 simulators 只能包含 Simulator"
        );
    }
}
