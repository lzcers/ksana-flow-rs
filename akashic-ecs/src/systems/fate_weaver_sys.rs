use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Query, Res, ResMut},
};

use agent::agent::Context;

use crate::{
    components::fate_weaver::{FateFrame, FateWeaver},
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
    utils::{parse_json_response, task_error_message, task_success_output, write_task_failed},
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

    let Some(spec) = fate_phase_spec(turn_state.phase) else {
        return;
    };

    let Some(context) = spec.build_context(
        fate_weaver,
        &world_state,
        turn_state.latest_protagonist_action.trim(),
    ) else {
        return;
    };

    task_manager.spawn_task(entity, spec.kind, &context);
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
    let Some(spec) = fate_phase_spec(turn_state.phase) else {
        task_manager.clear_task(entity);
        return;
    };

    // Fate 两段共用同一 entity，若上一段任务滞留到下一段，需要主动清理而不是继续消费错 phase 的结果。
    if task_result.kind != spec.kind {
        task_manager.clear_task(entity);
        return;
    }

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let raw_output = task_success_output(&task_result);

            let frame = match parse_json_response::<FateFrame>(&raw_output) {
                Ok(frame) => frame,
                Err(message) => {
                    write_task_failed(&mut event_writer, &turn_state, entity, message);
                    task_manager.clear_task(entity);
                    return;
                }
            };

            let fate_summary = frame.summary();
            let action = spec.action_text(&turn_state);

            world_state.apply_fate_frame(
                &frame,
                fate_weaver.protagonist_name(),
                turn_state.phase,
                action,
            );

            spec.write_success_event(&mut event_writer, turn_state.active_turn_id, fate_summary);
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "fate task failed");

            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
    }
}

#[derive(Clone, Copy)]
struct FatePhaseSpec {
    kind: TaskKind,
    needs_action: bool,
}

impl FatePhaseSpec {
    // 用 phase spec 统一收敛“当前阶段如何构造 Fate 上下文”的规则。
    fn build_context(
        self,
        fate_weaver: &FateWeaver,
        world_state: &WorldState,
        latest_action: &str,
    ) -> Option<Context> {
        match (self.kind, self.needs_action) {
            (TaskKind::FateScenePlanning, false) => {
                Some(fate_weaver.build_scene_context(world_state))
            }
            (TaskKind::FateConsequence, true) if !latest_action.is_empty() => {
                Some(fate_weaver.build_consequence_context(world_state, latest_action))
            }
            _ => None,
        }
    }

    fn action_text(self, turn_state: &TurnState) -> Option<&str> {
        self.needs_action
            .then_some(turn_state.latest_protagonist_action.trim())
            .filter(|action| !action.is_empty())
    }

    fn write_success_event(
        self,
        event_writer: &mut MessageWriter<TurnEvent>,
        turn_id: u64,
        fate_summary: String,
    ) {
        match self.kind {
            TaskKind::FateScenePlanning => {
                event_writer.write(TurnEvent::SceneFateGenerated {
                    turn_id,
                    scene_facts: fate_summary,
                });
            }
            TaskKind::FateConsequence => {
                event_writer.write(TurnEvent::ConsequenceFateGenerated {
                    turn_id,
                    consequence_facts: fate_summary,
                });
            }
            _ => {}
        }
    }
}

// 把业务 phase 映射为 Fate 执行规格，spawn/apply 两边共用，避免多处 match 漏改。
fn fate_phase_spec(phase: TurnPhase) -> Option<FatePhaseSpec> {
    match phase {
        TurnPhase::FateWeaving => Some(FatePhaseSpec {
            kind: TaskKind::FateScenePlanning,
            needs_action: false,
        }),
        TurnPhase::FateConsequence => Some(FatePhaseSpec {
            kind: TaskKind::FateConsequence,
            needs_action: true,
        }),
        _ => None,
    }
}

fn is_fate_task_kind(kind: TaskKind) -> bool {
    matches!(
        kind,
        TaskKind::FateScenePlanning | TaskKind::FateConsequence
    )
}
