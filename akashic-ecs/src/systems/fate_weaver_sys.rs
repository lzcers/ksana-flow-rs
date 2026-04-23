use agent::agent::{Context, LayerKind};
use bevy_ecs::{
    entity::Entity,
    message::{MessageReader, MessageWriter},
    system::{Query, Res, ResMut},
};
use serde_json::json;

use crate::{
    components::{
        fate_weaver::{FateFrame, FateLine, FateWeaver},
        protagonist::Protagonist,
        upper_narrator::UpperNarrator,
    },
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnCommand, TurnEvent},
    utils::{build_layer, parse_json_response},
};

// FateWeaver 只消费调度消息并发起任务。
pub fn fate_weaver_system(
    query: Query<(Entity, &FateWeaver)>,
    mut command_reader: MessageReader<TurnCommand>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, fate_weaver)) = query.single() else {
        return;
    };

    for command in command_reader.read() {
        if let TurnCommand::RequestFate { .. } = command {
            if task_manager.task_status(entity).is_none() {
                task_manager.spawn_task(entity, TaskKind::FateWeaving, fate_weaver.get_context());
            }
        }
    }
}

// Fate 结果解释与上下文回写放在独立 apply system，不放进回合状态机。
pub fn fate_result_apply_system(
    mut fate_weaver_query: Query<(Entity, &mut FateWeaver, &mut FateLine)>,
    mut protagonist_query: Query<&mut Protagonist>,
    mut upper_narrator_query: Query<&mut UpperNarrator>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::AwaitingFateResult {
        return;
    }

    let Ok((entity, mut fate_weaver, mut fate_line)) = fate_weaver_query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager
        .task_result(entity)
        .filter(|task_result| task_result.kind == TaskKind::FateWeaving)
    else {
        return;
    };

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let raw_output = task_result
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| task_result.chunks.join(""));

            let frame = match parse_json_response::<FateFrame>(&raw_output) {
                Ok(frame) => frame,
                Err(message) => {
                    event_writer.write(TurnEvent::TaskFailed {
                        turn_id: turn_state.active_turn_id,
                        stage: TurnPhase::AwaitingFateResult,
                        entity,
                        message,
                    });
                    task_manager.clear_task(entity);
                    return;
                }
            };

            let has_choices = !frame.choices.is_empty();
            let requires_protagonist = has_choices;
            let summary = frame.brief_summary();

            fate_line.push_frame(frame.clone());
            sync_fate_context(fate_weaver.get_context_mut(), &frame);

            if let Ok(mut protagonist) = protagonist_query.single_mut() {
                sync_fate_context(protagonist.get_context_mut(), &frame);
            }

            if let Ok(mut upper_narrator) = upper_narrator_query.single_mut() {
                sync_fate_context(upper_narrator.get_context_mut(), &frame);
            }

            event_writer.write(TurnEvent::FateWeavingCompleted {
                turn_id: turn_state.active_turn_id,
                requires_protagonist,
                has_choices,
                summary,
            });
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_result
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "fate task failed".to_string());

            event_writer.write(TurnEvent::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: TurnPhase::AwaitingFateResult,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}

fn sync_fate_context(context: &mut Context, frame: &FateFrame) {
    let memory_entries = json!([{ "content": frame.summary() }]);

    if let Some(layer) = context.get_mut("shared-fate-state") {
        layer.kind = LayerKind::Memory;
        layer.data = memory_entries;
        layer.meta.priority = 90;
    } else {
        context.layers.push(build_layer(
            "shared-fate-state",
            LayerKind::Memory,
            memory_entries,
            90,
        ));
    }
}
