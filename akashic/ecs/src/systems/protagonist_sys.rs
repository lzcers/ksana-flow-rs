use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::protagonist::{self, Protagonist},
    resources::{
        protagonist_action::{ProtagonistDecisionState, ProtagonistOptions},
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    turn_messages::TurnEvent,
    utils::{parse_json_response, task_error_message, task_success_output, write_task_failed},
};

pub fn protagonist_system(
    turn_state: Res<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    decision_state: Res<ProtagonistDecisionState>,
    mut query: Query<(Entity, &mut Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::ProtagonistReady {
        return;
    }

    let Ok((entity, mut protagonist)) = query.single_mut() else {
        return;
    };
    let protagonist_action = decision_state.committed_action();
    protagonist.add_user_message(&world_snapshot.to_protagonist_prompt(Some(protagonist_action)));
    task_manager.spawn_task(entity, TaskKind::ProtagonistAction, &protagonist.context());
    event_writer.write(TurnEvent::ProtagonistStarted);
}

pub fn protagonist_apply_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut Protagonist), With<protagonist::Protagonist>>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::ProtagonistRunning {
        return;
    }

    let Ok((entity, _)) = query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager.results.get(&entity).cloned() else {
        return;
    };

    match task_result.status {
        TaskStatus::Done => {
            let raw_output = task_success_output(&task_result);
            let action_list = match parse_json_response::<ProtagonistOptions>(&raw_output) {
                Ok(action_list) => action_list,
                Err(message) => {
                    write_task_failed(&mut event_writer, &turn_state, entity, message);
                    task_manager.clear_task(entity);
                    return;
                }
            };
            if action_list.is_empty() {
                write_task_failed(
                    &mut event_writer,
                    &turn_state,
                    entity,
                    "主角候选项为空".to_string(),
                );
                task_manager.clear_task(entity);
                return;
            }
            if let Ok((_, mut protagonist)) = query.single_mut() {
                protagonist.append_assistant_message(&raw_output);
            }
            decision_state.replace_with_options(action_list);
            event_writer.write(TurnEvent::ChoicesPrepared);
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "Protagonist 任务失败");
            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
        TaskStatus::Pending | TaskStatus::Running => {}
    }
}
