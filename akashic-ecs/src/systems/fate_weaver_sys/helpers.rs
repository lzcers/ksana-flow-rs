use agent::agent::Context;
use bevy_ecs::message::MessageWriter;

use crate::{
    components::fate_weaver::FateWeaver,
    resources::{
        task_manager::TaskKind,
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
};

pub(super) fn build_fate_context(
    phase: TurnPhase,
    fate_weaver: &FateWeaver,
    world_state: &WorldState,
    latest_action: &str,
) -> Option<Context> {
    match phase {
        TurnPhase::FateWeaving => Some(fate_weaver.build_scene_context(world_state)),
        TurnPhase::FateConsequence if !latest_action.is_empty() => {
            Some(fate_weaver.build_consequence_context(world_state, latest_action))
        }
        _ => None,
    }
}

pub(super) fn fate_action_text(phase: TurnPhase, turn_state: &TurnState) -> Option<&str> {
    match phase {
        TurnPhase::FateConsequence => {
            let action = turn_state.latest_protagonist_action.trim();
            (!action.is_empty()).then_some(action)
        }
        _ => None,
    }
}

pub(super) fn write_fate_success_event(
    phase: TurnPhase,
    event_writer: &mut MessageWriter<TurnEvent>,
    turn_id: u64,
    fate_summary: String,
) {
    match phase {
        TurnPhase::FateWeaving => {
            event_writer.write(TurnEvent::SceneFateGenerated {
                turn_id,
                scene_facts: fate_summary,
            });
        }
        TurnPhase::FateConsequence => {
            event_writer.write(TurnEvent::ConsequenceFateGenerated {
                turn_id,
                consequence_facts: fate_summary,
            });
        }
        _ => {}
    }
}

// 只保留 phase -> TaskKind 的轻量映射，避免 spawn/apply 两边各写一份判定。
pub(super) fn fate_task_kind(phase: TurnPhase) -> Option<TaskKind> {
    match phase {
        TurnPhase::FateWeaving => Some(TaskKind::FateScenePlanning),
        TurnPhase::FateConsequence => Some(TaskKind::FateConsequence),
        _ => None,
    }
}

pub(super) fn is_fate_task_kind(kind: TaskKind) -> bool {
    matches!(
        kind,
        TaskKind::FateScenePlanning | TaskKind::FateConsequence
    )
}
