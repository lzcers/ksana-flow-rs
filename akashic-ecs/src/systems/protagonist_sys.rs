use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::{fate_weaver::write_shared_fate_context, protagonist::Protagonist},
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::TurnEvent,
};

pub fn protagonist_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
) {
    if turn_state.phase != TurnPhase::AwaitingProtagonist {
        return;
    }

    let Ok((entity, mut protagonist)) = query.single_mut() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        let shared_facts = &turn_state.latest_fate_summary;
        if !shared_facts.trim().is_empty() {
            write_shared_fate_context(protagonist.get_context_mut(), shared_facts);
        }
        task_manager.spawn_task(entity, TaskKind::ProtagonistAction, protagonist.get_context());
    }
}

pub fn protagonist_result_apply_system(
    query: Query<Entity, With<Protagonist>>,
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

    if task_result.kind != TaskKind::ProtagonistAction {
        return;
    }

    if turn_state.phase != TurnPhase::AwaitingProtagonist {
        task_manager.clear_task(entity);
        return;
    }

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let action_text = task_result
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| task_result.chunks.join(""))
                .trim()
                .to_string();

            if action_text.is_empty() {
                event_writer.write(TurnEvent::TaskFailed {
                    turn_id: turn_state.active_turn_id,
                    stage: turn_state.phase,
                    entity,
                    message: "protagonist action is empty".to_string(),
                });
            } else {
                event_writer.write(TurnEvent::ProtagonistActionGenerated {
                    turn_id: turn_state.active_turn_id,
                    action_text,
                });
            }
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_result
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "protagonist task failed".to_string());

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
