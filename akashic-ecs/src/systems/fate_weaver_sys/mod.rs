use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Query, Res, ResMut},
};

use crate::{
    components::fate_weaver::{FateFrame, FateWeaver},
    resources::{
        task_manager::{TaskManager, TaskStatus},
        turn_state::TurnState,
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
    utils::{parse_json_response, task_error_message, task_success_output, write_task_failed},
};

mod helpers;

use self::helpers::{
    build_fate_context, fate_action_text, fate_task_kind, is_fate_task_kind,
    write_fate_success_event,
};

// FateWeaver 根据当前 phase 决定是做场景编排还是动作后果推演。
pub fn fate_weaver_system(
    turn_state: Res<TurnState>,
    world_state: Res<WorldState>,
    query: Query<(Entity, &FateWeaver)>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, fate_weaver)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_some() {
        return;
    }

    let Some(kind) = fate_task_kind(turn_state.phase) else {
        return;
    };

    let Some(context) = build_fate_context(
        turn_state.phase,
        fate_weaver,
        &world_state,
        turn_state.latest_protagonist_action.trim(),
    ) else {
        return;
    };

    task_manager.spawn_task(entity, kind, &context);
}

// Fate 结果解释与上下文回写放在独立 apply system，不放进回合状态机。
pub fn fate_result_apply_system(
    fate_weaver_query: Query<(Entity, &FateWeaver)>,
    turn_state: Res<TurnState>,
    mut world_state: ResMut<WorldState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    let Ok((entity, fate_weaver)) = fate_weaver_query.single() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    if !is_fate_task_kind(task_result.kind) {
        return;
    }

    // phase 已经漂移出 Fate，说明这个结果不再属于当前回合推进，直接清掉旧任务避免卡住后续 spawn。
    let Some(expected_kind) = fate_task_kind(turn_state.phase) else {
        task_manager.clear_task(entity);
        return;
    };

    // Fate 两段共用同一 entity，若上一段任务滞留到下一段，需要主动清理而不是继续消费错 phase 的结果。
    if task_result.kind != expected_kind {
        task_manager.clear_task(entity);
        return;
    }

    // 轮询任务，获取任务结果
    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let raw_output = task_success_output(&task_result);

            // 解析 JSON 输出为 FateFrame 结构体
            let frame = match parse_json_response::<FateFrame>(&raw_output) {
                Ok(frame) => frame,
                Err(message) => {
                    write_task_failed(&mut event_writer, &turn_state, entity, message);
                    task_manager.clear_task(entity);
                    return;
                }
            };

            // 根据结构体生成文本描述
            let fate_summary = frame.summary();
            //  抽取玩家动作，如果当前是 FateConsequence phase，且玩家有输入动作
            let action = fate_action_text(turn_state.phase, &turn_state);

            // 应用 FateFrame 到世界状态
            world_state.apply_fate_frame(
                &frame,
                fate_weaver.protagonist_name(),
                turn_state.phase,
                action,
            );

            // 通知 orchestrator 任务完成
            write_fate_success_event(
                turn_state.phase,
                &mut event_writer,
                turn_state.active_turn_id,
                fate_summary,
            );
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "fate task failed");

            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
    }
}
