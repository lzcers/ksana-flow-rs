use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::World,
    schedule::{IntoScheduleConfigs, Schedule},
};

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PendingProtagonistChoice, ProtagonistDecisionState},
        story_state::LatestNarration,
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    systems::{
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceResolutionMode {
    AutoSelectFirst,
    WaitForUser,
}

#[derive(Debug, Clone)]
pub struct AkashicSessionSnapshot {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub protagonist_action: String,
    pub world_snapshot: WorldSnapshot,
    pub latest_narration: String,
    pub choices: Vec<PendingProtagonistChoice>,
    pub finished: bool,
}

pub struct AkashicSessionEngine {
    world: World,
    schedule: Schedule,
    max_turns: u64,
    choice_resolution_mode: ChoiceResolutionMode,
}

impl AkashicSessionEngine {
    pub fn new() -> Self {
        Self::new_with_profiles_and_mode(
            DEFAULT_WORLD_PROFILE,
            DEFAULT_PROTAGONIST_PROFILE,
            ChoiceResolutionMode::AutoSelectFirst,
        )
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
        choice_resolution_mode: ChoiceResolutionMode,
    ) -> Self {
        let mut world = World::new();
        world.init_resource::<WorldSnapshot>();
        world.init_resource::<TurnState>();
        world.init_resource::<PlayerInbox>();
        world.insert_resource(player_input_config(choice_resolution_mode));
        world.init_resource::<ProtagonistDecisionState>();
        world.init_resource::<LatestNarration>();
        world.init_resource::<Messages<TurnEvent>>();
        world.init_resource::<Messages<TurnControl>>();
        world.insert_resource(TaskManager::new(build_chat_model()));

        world.spawn(FateWeaver::new(world_profile, protagonist_profile));
        world.spawn(UpperNarrator::new(world_profile, protagonist_profile));
        world.spawn(Protagonist::new(world_profile, protagonist_profile));

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
            )
                .chain(),
        );

        Self {
            world,
            schedule,
            max_turns: 3,
            choice_resolution_mode,
        }
    }

    pub fn bootstrap(&mut self) -> Result<AkashicSessionSnapshot, String> {
        self.run_until_stable(false)
    }

    pub fn submit_choice(&mut self, choice_id: &str) -> Result<AkashicSessionSnapshot, String> {
        let snapshot = self.snapshot();
        if snapshot.phase != TurnPhase::AwaitingPlayerChoice {
            return Err("当前会话尚未进入等待玩家选择阶段。".to_string());
        }

        snapshot
            .choices
            .iter()
            .find(|choice| choice.id == choice_id)
            .ok_or_else(|| format!("所选分支 `{choice_id}` 不存在或已失效。"))?;

        self.world
            .resource_mut::<PlayerInbox>()
            .push(PlayerCommand::SubmitChoice {
                turn_id: snapshot.active_turn_id,
                choice_id: choice_id.to_string(),
            });

        self.run_until_stable(true)
    }

    pub fn snapshot(&self) -> AkashicSessionSnapshot {
        let turn_state = self.world.resource::<TurnState>();
        let decision_state = self.world.resource::<ProtagonistDecisionState>();
        let protagonist_action = decision_state.committed_action().to_string();
        let world_snapshot = self.world.resource::<WorldSnapshot>().clone();
        let latest_narration = self.world.resource::<LatestNarration>().0.clone();
        let choices = decision_state.choices().to_vec();
        let finished = turn_state.phase == TurnPhase::Idle && turn_state.turn_index >= self.max_turns;

        AkashicSessionSnapshot {
            phase: turn_state.phase,
            turn_index: turn_state.turn_index,
            active_turn_id: turn_state.active_turn_id,
            protagonist_action,
            world_snapshot,
            latest_narration,
            choices,
            finished,
        }
    }

    fn run_until_stable(&mut self, force_tick: bool) -> Result<AkashicSessionSnapshot, String> {
        if force_tick {
            self.schedule.run(&mut self.world);
        }

        loop {
            let snapshot = self.snapshot();
            if snapshot.phase == TurnPhase::Failed {
                return Err("故事生成失败，ECS 进入 Failed 状态。".to_string());
            }
            if snapshot.finished {
                return Ok(snapshot);
            }
            if snapshot.phase == TurnPhase::AwaitingPlayerChoice && self.task_queue_empty() {
                if self.choice_resolution_mode == ChoiceResolutionMode::WaitForUser {
                    return Ok(snapshot);
                }
            }

            self.schedule.run(&mut self.world);
        }
    }

    fn task_queue_empty(&self) -> bool {
        self.world
            .resource::<TaskManager>()
            .task_results_snapshot()
            .is_empty()
    }
}

fn player_input_config(choice_resolution_mode: ChoiceResolutionMode) -> PlayerInputConfig {
    match choice_resolution_mode {
        ChoiceResolutionMode::AutoSelectFirst => PlayerInputConfig::auto_select_first(),
        ChoiceResolutionMode::WaitForUser => PlayerInputConfig::wait_for_user(),
    }
}
