use std::{thread, time::Duration};

use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, watch};

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        export::{ExportHandle, ExportState, SessionEvent, SessionSnapshot, TaskView},
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PendingProtagonistChoice, ProtagonistDecisionState},
        task_manager::{TaskManager, TaskStatus},
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

const DEFAULT_MAX_ADVANCE_STEPS: usize = 20_000;
const DEFAULT_SPIN_WAIT: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceResolutionMode {
    WaitForUser,
    AutoSelectFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivePlayer {
    System,
    FateWeaver,
    Narrator,
    Protagonist,
    ExternalPlayer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AkashicSessionSnapshot {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub current_player: ActivePlayer,
    pub current_task: Option<TaskView>,
    pub tasks: Vec<TaskView>,
    pub world_snapshot: WorldSnapshot,
    pub latest_narration: String,
    pub current_protagonist_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
    pub choice_resolution_mode: ChoiceResolutionMode,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineCommand {
    Tick,
    ContinueTurn { turn_id: Option<u64> },
    ResetTurn { next_turn_id: Option<u64> },
    SubmitChoice { choice_id: String },
    SwitchPlayer { mode: ChoiceResolutionMode },
}

pub struct AkashicSessionEngine {
    world: World,
    schedule: Schedule,
    export_handle: ExportHandle,
    choice_resolution_mode: ChoiceResolutionMode,
    max_advance_steps: usize,
    spin_wait: Duration,
}

impl Default for ChoiceResolutionMode {
    fn default() -> Self {
        Self::WaitForUser
    }
}

impl ChoiceResolutionMode {
    fn player_input_config(self) -> PlayerInputConfig {
        match self {
            Self::WaitForUser => PlayerInputConfig::wait_for_user(),
            Self::AutoSelectFirst => PlayerInputConfig::auto_select_first(),
        }
    }
}

impl AkashicSessionEngine {
    pub fn new() -> Self {
        Self::new_with_profiles(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE)
    }

    pub fn new_with_mode(mode: ChoiceResolutionMode) -> Self {
        Self::new_with_profiles_and_mode(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE, mode)
    }

    pub fn new_with_profiles(world_profile: &str, protagonist_profile: &str) -> Self {
        Self::new_with_profiles_and_mode(
            world_profile,
            protagonist_profile,
            ChoiceResolutionMode::WaitForUser,
        )
    }

    pub fn new_with_profiles_and_mode(
        world_profile: &str,
        protagonist_profile: &str,
        mode: ChoiceResolutionMode,
    ) -> Self {
        let (world, export_handle) = build_world(world_profile, protagonist_profile, mode);
        Self {
            world,
            schedule: build_schedule(),
            export_handle,
            choice_resolution_mode: mode,
            max_advance_steps: DEFAULT_MAX_ADVANCE_STEPS,
            spin_wait: DEFAULT_SPIN_WAIT,
        }
    }

    pub fn bootstrap(&mut self) -> Result<AkashicSessionSnapshot, String> {
        self.advance_until_external_checkpoint(false)
    }

    pub fn snapshot(&mut self) -> AkashicSessionSnapshot {
        self.build_snapshot()
    }

    pub fn current_export_snapshot(&self) -> SessionSnapshot {
        self.export_handle.current_snapshot()
    }

    pub fn snapshot_receiver(&self) -> watch::Receiver<SessionSnapshot> {
        self.export_handle.snapshot_receiver()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<SessionEvent> {
        self.export_handle.subscribe_events()
    }

    pub fn tick(&mut self) -> Result<AkashicSessionSnapshot, String> {
        self.run_one_frame()?;
        Ok(self.build_snapshot())
    }

    pub fn continue_turn(&mut self) -> Result<AkashicSessionSnapshot, String> {
        let turn_id = self.world.resource::<TurnState>().active_turn_id;
        self.enqueue_turn_control(TurnControl::ContinueTurn { turn_id });
        self.advance_until_external_checkpoint(false)
    }

    pub fn reset_turn(
        &mut self,
        next_turn_id: Option<u64>,
    ) -> Result<AkashicSessionSnapshot, String> {
        self.enqueue_turn_control(TurnControl::ResetTurn { next_turn_id });
        self.advance_until_external_checkpoint(false)
    }

    pub fn submit_choice(&mut self, choice_id: &str) -> Result<AkashicSessionSnapshot, String> {
        let turn_id = self.world.resource::<TurnState>().active_turn_id;
        self.world
            .resource_mut::<PlayerInbox>()
            .push(PlayerCommand::SubmitChoice {
                turn_id,
                choice_id: choice_id.to_string(),
            });
        self.advance_until_external_checkpoint(true)
    }

    pub fn switch_player(
        &mut self,
        mode: ChoiceResolutionMode,
    ) -> Result<AkashicSessionSnapshot, String> {
        self.choice_resolution_mode = mode;
        *self.world.resource_mut::<PlayerInputConfig>() = mode.player_input_config();
        self.advance_until_external_checkpoint(false)
    }

    pub fn apply_command(
        &mut self,
        command: EngineCommand,
    ) -> Result<AkashicSessionSnapshot, String> {
        match command {
            EngineCommand::Tick => self.tick(),
            EngineCommand::ContinueTurn { turn_id } => {
                let turn_id =
                    turn_id.unwrap_or_else(|| self.world.resource::<TurnState>().active_turn_id);
                self.enqueue_turn_control(TurnControl::ContinueTurn { turn_id });
                self.advance_until_external_checkpoint(false)
            }
            EngineCommand::ResetTurn { next_turn_id } => self.reset_turn(next_turn_id),
            EngineCommand::SubmitChoice { choice_id } => self.submit_choice(&choice_id),
            EngineCommand::SwitchPlayer { mode } => self.switch_player(mode),
        }
    }

    fn advance_until_external_checkpoint(
        &mut self,
        auto_continue_finished_turn: bool,
    ) -> Result<AkashicSessionSnapshot, String> {
        for _ in 0..self.max_advance_steps {
            self.run_one_frame()?;
            let snapshot = self.build_snapshot();

            if snapshot.phase == TurnPhase::TurnFinished && auto_continue_finished_turn {
                self.enqueue_turn_control(TurnControl::ContinueTurn {
                    turn_id: snapshot.active_turn_id,
                });
                continue;
            }

            if is_external_checkpoint(&snapshot) {
                return Ok(snapshot);
            }

            if snapshot.current_task.is_some() {
                thread::sleep(self.spin_wait);
            }
        }

        Err(format!(
            "引擎在 {} 帧内未到达可对外暴露的稳定状态",
            self.max_advance_steps
        ))
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

    fn build_snapshot(&mut self) -> AkashicSessionSnapshot {
        let export_snapshot = self.export_handle.current_snapshot();
        let decision_state = self.world.resource::<ProtagonistDecisionState>().clone();
        let latest_narration = self.latest_narration();
        let current_task = select_current_task(&export_snapshot.tasks);

        AkashicSessionSnapshot {
            phase: export_snapshot.turn.phase,
            turn_index: export_snapshot.turn.turn_index,
            active_turn_id: export_snapshot.turn.active_turn_id,
            current_player: active_player_for(
                export_snapshot.turn.phase,
                self.choice_resolution_mode,
            ),
            current_task,
            tasks: export_snapshot.tasks,
            world_snapshot: export_snapshot.world,
            latest_narration,
            current_protagonist_action: decision_state.committed_action().to_string(),
            choices: decision_state.choices().to_vec(),
            choice_resolution_mode: self.choice_resolution_mode,
            finished: false,
        }
    }

    fn latest_narration(&mut self) -> String {
        let mut query = self.world.query::<&UpperNarrator>();
        query
            .iter(&self.world)
            .next()
            .and_then(|narrator| narrator.context().print_latest_msg())
            .unwrap_or_default()
    }

    fn enqueue_turn_control(&mut self, control: TurnControl) {
        self.world
            .resource_mut::<Messages<TurnControl>>()
            .write(control);
    }
}

fn build_world(
    world_profile: &str,
    protagonist_profile: &str,
    mode: ChoiceResolutionMode,
) -> (World, ExportHandle) {
    let mut world = World::new();

    world.init_resource::<WorldSnapshot>();
    world.init_resource::<TurnState>();
    world.init_resource::<PlayerInbox>();
    world.insert_resource(mode.player_input_config());
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

fn select_current_task(tasks: &[TaskView]) -> Option<TaskView> {
    tasks
        .iter()
        .find(|task| matches!(task.status, TaskStatus::Running))
        .or_else(|| {
            tasks
                .iter()
                .find(|task| matches!(task.status, TaskStatus::Pending))
        })
        .cloned()
}

fn active_player_for(phase: TurnPhase, mode: ChoiceResolutionMode) -> ActivePlayer {
    match phase {
        TurnPhase::Idle | TurnPhase::TurnFinished | TurnPhase::Failed => ActivePlayer::System,
        TurnPhase::FateWeaving => ActivePlayer::FateWeaver,
        TurnPhase::NarratorWriting | TurnPhase::NarratorStory => ActivePlayer::Narrator,
        TurnPhase::ProtagonistAction | TurnPhase::AwaitingProtagonist => ActivePlayer::Protagonist,
        TurnPhase::AwaitingPlayerChoice => match mode {
            ChoiceResolutionMode::WaitForUser => ActivePlayer::ExternalPlayer,
            ChoiceResolutionMode::AutoSelectFirst => ActivePlayer::Protagonist,
        },
    }
}

fn is_external_checkpoint(snapshot: &AkashicSessionSnapshot) -> bool {
    matches!(
        snapshot.phase,
        TurnPhase::AwaitingPlayerChoice | TurnPhase::TurnFinished
    ) || (snapshot.phase == TurnPhase::Idle && snapshot.current_task.is_none())
}
