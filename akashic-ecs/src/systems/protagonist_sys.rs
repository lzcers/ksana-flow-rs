use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use agent::agent::Context;

use crate::{
    components::protagonist::Protagonist,
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
    utils::{task_error_message, task_success_output, write_task_failed},
};

pub fn protagonist_system(
    turn_state: Res<TurnState>,
    world_state: Res<WorldState>,
    query: Query<(Entity, &Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, protagonist)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_some() {
        return;
    }

    let Some(spec) = protagonist_task_spec(turn_state.phase) else {
        return;
    };

    let context = spec.build_context(protagonist, &world_state);
    task_manager.spawn_task(entity, spec.kind, &context);
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

    // phase 已经离开主角决策阶段时，旧任务结果不再参与当前推进，直接清理避免阻塞下一次 spawn。
    let Some(spec) = protagonist_task_spec(turn_state.phase) else {
        task_manager.clear_task(entity);
        return;
    };

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let action_text = task_success_output(&task_result).trim().to_string();

            if action_text.is_empty() {
                write_task_failed(
                    &mut event_writer,
                    &turn_state,
                    entity,
                    "protagonist action is empty".to_string(),
                );
            } else {
                spec.write_success_event(&mut event_writer, turn_state.active_turn_id, action_text);
            }
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "protagonist task failed");

            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
    }
}

#[derive(Clone, Copy)]
struct ProtagonistTaskSpec {
    kind: TaskKind,
}

impl ProtagonistTaskSpec {
    fn build_context(self, protagonist: &Protagonist, world_state: &WorldState) -> Context {
        protagonist.build_task_context(world_state)
    }

    fn write_success_event(
        self,
        event_writer: &mut MessageWriter<TurnEvent>,
        turn_id: u64,
        action_text: String,
    ) {
        let _ = self;
        event_writer.write(TurnEvent::ProtagonistActionGenerated {
            turn_id,
            action_text,
        });
    }
}

// 主角目前只在 AwaitingProtagonist 阶段工作，但保留 spec 入口方便和其它 agent 统一模板。
fn protagonist_task_spec(phase: TurnPhase) -> Option<ProtagonistTaskSpec> {
    match phase {
        TurnPhase::AwaitingProtagonist => Some(ProtagonistTaskSpec {
            kind: TaskKind::ProtagonistAction,
        }),
        _ => None,
    }
}
