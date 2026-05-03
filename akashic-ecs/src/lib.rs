pub mod components;
pub mod profile;
pub mod prompts;
pub mod resources;
pub mod systems;
pub mod turn_messages;
pub mod ui;
pub mod utils;

pub use ui::UiEvent;

use bevy_ecs::{
    message::message_update_system,
    schedule::{IntoScheduleConfigs, Schedule},
    world::World,
};

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        manual_control::ManualControlState,
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    systems::{
        fate_weaver_sys::{fate_result_apply_system, fate_weaver_system},
        protagonist_sys::{protagonist_result_apply_system, protagonist_system},
        task_sys::task_system,
        turn_orchestrator_sys::turn_orchestrator_system,
        upper_narrator_sys::{upper_narrator_result_apply_system, upper_narrator_system},
    },
    turn_messages::{TurnControl, register_turn_messages},
    ui::{UiEventBuffer, ui_bridge_system},
    utils::build_chat_model,
};

pub struct AkashicRuntime {
    world: World,
    schedule: Schedule,
}

#[derive(Debug, Clone)]
pub struct RuntimeSummary {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub latest_history: String,
    pub latest_protagonist_action: String,
    pub latest_broadcast_summary: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeStateView {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub latest_broadcast_summary: String,
    pub latest_protagonist_action: String,
    pub current_location: String,
    pub current_scene: String,
    pub protagonist_state: String,
    pub npcs_state: String,
    pub latest_history: String,
}

impl AkashicRuntime {
    pub fn new() -> Self {
        let mut world = World::new();
        register_turn_messages(&mut world);
        world.insert_resource(ManualControlState::default());
        world.insert_resource(TaskManager::new(build_chat_model()));
        world.insert_resource(TurnState::default());
        world.insert_resource(WorldState::default());
        world.insert_resource(UiEventBuffer::default());

        world.spawn((FateWeaver::new(
            DEFAULT_WORLD_PROFILE,
            DEFAULT_PROTAGONIST_PROFILE,
        ),));
        world.spawn((Protagonist::new(DEFAULT_PROTAGONIST_PROFILE),));
        world.spawn((UpperNarrator::new(),));

        world.resource_mut::<TurnState>().start_turn(1);

        Self {
            world,
            schedule: build_schedule(),
        }
    }

    pub fn tick(&mut self) {
        self.schedule.run(&mut self.world);
    }

    pub fn drain_ui_events(&mut self) -> Vec<UiEvent> {
        self.world.resource_mut::<UiEventBuffer>().drain()
    }

    pub fn phase(&self) -> TurnPhase {
        self.world.resource::<TurnState>().phase
    }

    pub fn turn_index(&self) -> u64 {
        self.world.resource::<TurnState>().turn_index
    }

    pub fn active_turn_id(&self) -> u64 {
        self.world.resource::<TurnState>().active_turn_id
    }

    pub fn restart_turn(&mut self) {
        let next_turn_id = self.active_turn_id() + 1;
        let mut turn_state = self.world.resource_mut::<TurnState>();
        turn_state.reset(Some(next_turn_id));
        turn_state.start_turn(next_turn_id);
    }

    pub fn submit_protagonist_action(
        &mut self,
        action_text: impl Into<String>,
    ) -> Result<(), String> {
        let action_text = action_text.into();
        if action_text.trim().is_empty() {
            return Err("动作不能为空".to_string());
        }

        let turn_id = {
            let turn_state = self.world.resource::<TurnState>();
            if turn_state.phase != TurnPhase::AwaitingProtagonist {
                return Err("当前不在等待主角决策阶段".to_string());
            }
            turn_state.active_turn_id
        };
        self.world
            .write_message(TurnControl::SubmitProtagonistAction {
                turn_id,
                action_text,
            })
            .ok_or_else(|| "提交主角动作失败".to_string())?;
        self.world
            .resource_mut::<ManualControlState>()
            .pending_protagonist_override_turn = Some(turn_id);
        Ok(())
    }

    pub fn can_submit_protagonist_action(&self) -> bool {
        self.phase() == TurnPhase::AwaitingProtagonist
    }

    pub fn summary(&self) -> RuntimeSummary {
        let turn_state = self.world.resource::<TurnState>();
        let world_state = self.world.resource::<WorldState>();

        RuntimeSummary {
            phase: turn_state.phase,
            turn_index: turn_state.turn_index,
            latest_history: world_state.latest_history_entry_text(),
            latest_protagonist_action: turn_state.latest_protagonist_action.trim().to_string(),
            latest_broadcast_summary: turn_state.latest_broadcast_summary.trim().to_string(),
        }
    }

    pub fn state_view(&self) -> RuntimeStateView {
        let turn_state = self.world.resource::<TurnState>();
        let world_state = self.world.resource::<WorldState>();

        RuntimeStateView {
            phase: turn_state.phase,
            turn_index: turn_state.turn_index,
            active_turn_id: turn_state.active_turn_id,
            latest_broadcast_summary: turn_state.latest_broadcast_summary.trim().to_string(),
            latest_protagonist_action: turn_state.latest_protagonist_action.trim().to_string(),
            current_location: world_state.current_location_text(),
            current_scene: world_state.current_scene_text(),
            protagonist_state: world_state.protagonist_state_text(),
            npcs_state: world_state.npcs_state_text(),
            latest_history: world_state.latest_history_entry_text(),
        }
    }
}

impl Default for AkashicRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            message_update_system,
            fate_weaver_system,
            protagonist_system,
            upper_narrator_system,
            task_system,
            fate_result_apply_system,
            protagonist_result_apply_system,
            upper_narrator_result_apply_system,
            ui_bridge_system,
            turn_orchestrator_system,
        )
            .chain(),
    );
    schedule
}
