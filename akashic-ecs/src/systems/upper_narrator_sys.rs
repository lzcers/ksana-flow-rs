use agent::{agent::Context, core::Message};
use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Query, Res, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::{
        protagonist_action::ProtagonistAction,
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    turn_messages::TurnEvent,
    utils::{task_error_message, task_success_output, write_task_failed},
};

pub fn upper_narrator_system(
    mut query: Query<(Entity, &mut UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
    turn_state: Res<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    mut protagonist_action: ResMut<ProtagonistAction>,
) {
    if turn_state.phase != TurnPhase::NarratorStory {
        return;
    }

    let Ok((entity, mut upper_narrator)) = query.single_mut() else {
        return;
    };
    let protagonist_action = protagonist_action.take();
    upper_narrator.add_user_message(&world_snapshot.to_story_prompt(Some(&protagonist_action)));
    task_manager.spawn_task(entity, TaskKind::Narration, &upper_narrator.context());
    event_writer.write(TurnEvent::StoryWriting);
}

pub fn upper_narrator_apply_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::NarratorWriting {
        return;
    }

    let Ok((entity, mut upper_narrator)) = query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    match task_result.status {
        TaskStatus::Done => {
            let story_content = task_success_output(&task_result);
            upper_narrator.append_assistant_message(&story_content);
            event_writer.write(TurnEvent::StoryGenerated);
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "UpperNarrator 任务失败");
            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
        TaskStatus::Pending | TaskStatus::Running => {}
    }
}
