use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Query, Res, ResMut},
};
use serde_json::json;

use crate::{
    components::fate_weaver::FateWeaver,
    resources::{
        protagonist_action::ProtagonistAction,
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    turn_messages::TurnEvent,
    utils::{parse_json_response, task_success_output},
};

// FateWeaver 根据当前 phase 决定是做场景编排还是动作后果推演。
pub fn fate_weaver_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut FateWeaver)>,
    mut protagonist_action: ResMut<ProtagonistAction>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    // 仅在 Idle phase 才执行。
    if turn_state.phase != TurnPhase::Idle {
        return;
    }
    let Ok((entity, mut fate_weaver)) = query.single_mut() else {
        return;
    };

    // let protagonist_action
    // 创建命运推演任务
    let round = turn_state.turn_index;
    let protagonist_action = protagonist_action.take();
    fate_weaver.append_user_message(
        &json!({"round": round, "protagonist_action": protagonist_action}).to_string(),
    );
    task_manager.spawn_task(entity, TaskKind::FatePlanning, &fate_weaver.context());
    // 发送场景 fate 规划事件
    event_writer.write(TurnEvent::FatePlanning);
}

pub fn fate_weaver_apply_system(
    turn_state: Res<TurnState>,
    mut query: Query<(Entity, &mut FateWeaver)>,
    mut world_snapshot: ResMut<WorldSnapshot>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::FateWeaving {
        return;
    }

    let Ok((entity, mut fate_weaver)) = query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    // 解析命运推演结果
    match task_result.status {
        TaskStatus::Done => {
            let raw_output = task_success_output(&task_result);
            let next_world_snapshot = parse_json_response::<WorldSnapshot>(&raw_output)
                .unwrap_or_else(|err| panic!("解析 FateWeaver 输出失败: {err}"));
            let ledger = next_world_snapshot.to_ledger();
            *world_snapshot = next_world_snapshot;
            fate_weaver.append_assistant_message(&ledger);
            event_writer.write(TurnEvent::FateGenerated);
            task_manager.clear_task(entity);
        }
        TaskStatus::Pending | TaskStatus::Running | TaskStatus::Error => {}
    }
}
