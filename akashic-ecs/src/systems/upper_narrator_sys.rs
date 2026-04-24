use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use agent::agent::Context;

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
    utils::{task_error_message, task_success_output, write_task_failed},
};

pub fn upper_narrator_system(
    turn_state: Res<TurnState>,
    world_state: Res<WorldState>,
    query: Query<(Entity, &UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, upper_narrator)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_some() {
        return;
    }

    let Some(spec) = narrator_phase_spec(turn_state.phase) else {
        return;
    };

    let context = spec.build_context(upper_narrator, &world_state);
    task_manager.spawn_task(entity, spec.kind, &context);
}

pub fn upper_narrator_result_apply_system(
    query: Query<Entity, With<UpperNarrator>>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    let Ok(entity) = query.single() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    if task_result.kind != TaskKind::Narration {
        return;
    }

    // 叙事阶段已经切走时，旧 narration task 不应继续占住 entity。
    let Some(spec) = narrator_phase_spec(turn_state.phase) else {
        task_manager.clear_task(entity);
        return;
    };

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let narration = task_success_output(&task_result);

            spec.write_success_event(&mut event_writer, turn_state.active_turn_id, narration);
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "narration task failed");

            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
    }
}

#[derive(Clone, Copy)]
struct NarratorPhaseSpec {
    phase: TurnPhase,
    kind: TaskKind,
}

impl NarratorPhaseSpec {
    fn build_context(self, upper_narrator: &UpperNarrator, world_state: &WorldState) -> Context {
        upper_narrator.build_task_context(world_state, self.phase)
    }

    fn write_success_event(
        self,
        event_writer: &mut MessageWriter<TurnEvent>,
        turn_id: u64,
        narration: String,
    ) {
        match self.phase {
            TurnPhase::NarratorScene => {
                event_writer.write(TurnEvent::SceneNarrationGenerated {
                    turn_id,
                    scene_text: narration,
                });
            }
            TurnPhase::NarratorStory => {
                event_writer.write(TurnEvent::StoryNarrationGenerated {
                    turn_id,
                    story_text: narration,
                });
            }
            _ => {}
        }
    }
}

// 把叙事阶段映射为统一规格，spawn/apply 共用，避免 Scene/Story 两处分支长期漂移。
fn narrator_phase_spec(phase: TurnPhase) -> Option<NarratorPhaseSpec> {
    match phase {
        TurnPhase::NarratorScene | TurnPhase::NarratorStory => Some(NarratorPhaseSpec {
            phase,
            kind: TaskKind::Narration,
        }),
        _ => None,
    }
}
