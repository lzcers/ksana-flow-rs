use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::protagonist::{self, Protagonist},
    resources::{
        protagonist_action::{ProtagonistAction, ProtagonistOptions},
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
    mut query: Query<(Entity, &mut Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::ProtagonistAction {
        return;
    }

    let Ok((entity, mut protagonist)) = query.single_mut() else {
        return;
    };
    protagonist.add_user_message(&world_snapshot.to_protagonist_prompt());
    task_manager.spawn_task(entity, TaskKind::ProtagonistAction, &protagonist.context());
    event_writer.write(TurnEvent::AwaitingProtagonist);
}

pub fn protagonist_apply_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut Protagonist), With<protagonist::Protagonist>>,
    mut protagonist_action: ResMut<ProtagonistAction>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::AwaitingProtagonist {
        return;
    }

    let Ok((entity, _)) = query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
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
            // 默认选第一个选项
            // 提交选项到 ProtagonistAction 资源
            protagonist_action.set(action_list.first_action().unwrap_or("continue").to_string());
            event_writer.write(TurnEvent::ProtagonistCompleted);
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
