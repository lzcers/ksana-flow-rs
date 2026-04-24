use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::{fate_weaver::write_shared_fate_context, upper_narrator::UpperNarrator},
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::TurnEvent,
};

pub fn upper_narrator_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
) {
    if !matches!(
        turn_state.phase,
        TurnPhase::NarratorScene | TurnPhase::NarratorStory
    ) {
        return;
    }

    let Ok((entity, mut upper_narrator)) = query.single_mut() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        let shared_facts = &turn_state.latest_fate_summary;
        if !shared_facts.trim().is_empty() {
            write_shared_fate_context(upper_narrator.get_context_mut(), shared_facts);
        }
        task_manager.spawn_task(entity, TaskKind::Narration, upper_narrator.get_context());
    }
}

pub fn upper_narrator_result_apply_system(
    query: Query<Entity, With<UpperNarrator>>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if !matches!(
        turn_state.phase,
        TurnPhase::NarratorScene | TurnPhase::NarratorStory
    ) {
        return;
    }

    let Ok(entity) = query.single() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    if task_result.kind != TaskKind::Narration {
        return;
    }

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let narration = task_result
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| task_result.chunks.join(""));

            match turn_state.phase {
                TurnPhase::NarratorScene => {
                    event_writer.write(TurnEvent::SceneNarrationGenerated {
                        turn_id: turn_state.active_turn_id,
                        scene_text: narration,
                    });
                }
                TurnPhase::NarratorStory => {
                    event_writer.write(TurnEvent::StoryNarrationGenerated {
                        turn_id: turn_state.active_turn_id,
                        story_text: narration,
                    });
                }
                _ => {}
            }
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_result
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "narration task failed".to_string());

            event_writer.write(TurnEvent::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: turn_state.phase,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}
