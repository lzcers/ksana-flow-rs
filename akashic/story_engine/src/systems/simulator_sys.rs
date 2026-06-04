use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};
use serde_json::json;

use crate::{
    components::{
        agent::{
            Agent, AgentOutputKind, OwnedAgentMut, PendingReasoning, ReadyAgentFilter,
            RunningReasoning, SessionOwner, SimulationOutcome,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

pub fn simulator_dispatch_system(
    mut commands: Commands,
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &ProtagonistDecisionState,
        &WorldSnapshot,
    )>,
    mut agents: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    // 检索所有 session 中处于 SimulationReady
    for (session_entity, mut flow, decision_state, world_snapshot) in sessions
        .iter_mut()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::SimulationReady)
    {
        let mut dispatched = 0;
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity && agent.output_kind.is_simulation()
        }) {
            agent.append_user_message(
                &json!({
                    "round": flow.active_turn_id.max(flow.turn_index + 1),
                    "protagonist_action": decision_state.committed_action(),
                    "previous_world_snapshot": world_snapshot,
                })
                .to_string(),
            );
            commands.entity(entity).insert(PendingReasoning);
            dispatched += 1;
        }
        // 当至少有一个 Entity 触发推理时，推动 flow 进入下一阶段
        flow.stage = if dispatched > 0 {
            TurnStage::SimulationRunning
        } else {
            TurnStage::Failed
        };
    }
}

pub fn simulator_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut WorldSnapshot)>,
    mut agents: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow, mut world_snapshot) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::SimulationRunning)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity && agent.output_kind.is_simulation()
        }) {
            let Some(result) = task_manager.task_result(entity).cloned() else {
                continue;
            };
            match result.status {
                TaskStatus::Done => {
                    let Some(output) = task_manager
                        .take_result(entity)
                        .and_then(|result| result.output)
                    else {
                        continue;
                    };

                    // 解析 Entity 输出的结果
                    if agent.output_kind == AgentOutputKind::WorldSnapshot {
                        let Ok(snapshot) = parse_json_response::<WorldSnapshot>(&output) else {
                            agent.revert();
                            commands.entity(entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::Failed;
                            break;
                        };
                        *world_snapshot = snapshot;
                    }
                    agent.append_assistant_message(&output);
                    commands.entity(entity).remove::<RunningReasoning>().insert(
                        SimulationOutcome {
                            turn_id: flow.active_turn_id,
                            content: output,
                        },
                    );
                }
                TaskStatus::Error => {
                    task_manager.clear_task(entity);
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn simulator_progress_system(
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    agents: Query<(
        &Agent,
        &SessionOwner,
        Option<&PendingReasoning>,
        Option<&RunningReasoning>,
        Option<&SimulationOutcome>,
    )>,
) {
    for (session_entity, mut flow) in sessions
        .iter_mut()
        .filter(|(_, flow)| flow.stage == TurnStage::SimulationRunning)
    {
        let mut simulator_count = 0;
        let mut completed_count = 0;
        let mut has_inflight = false;
        for (_, _, pending, running, outcome) in agents.iter().filter(|(agent, owner, ..)| {
            owner.0 == session_entity && agent.output_kind.is_simulation()
        }) {
            simulator_count += 1;
            has_inflight |= pending.is_some() || running.is_some();
            completed_count +=
                usize::from(outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id));
        }

        if flow.stage != TurnStage::Failed
            && simulator_count > 0
            && !has_inflight
            && completed_count == simulator_count
        {
            flow.stage = TurnStage::NarrationReady;
        }
    }
}

#[cfg(test)]
mod tests {
    use agent::agent::Context;
    use bevy_ecs::{schedule::Schedule, world::World};

    use super::*;
    use crate::components::agent::AgentOutputKind;

    fn simulator(name: &str) -> Agent {
        Agent::from_context(AgentOutputKind::SimulationText, name, Context::default())
    }

    #[test]
    fn advances_only_after_all_simulators_finish() {
        let mut world = World::new();
        let session = world
            .spawn(TurnFlow {
                active_turn_id: 1,
                stage: TurnStage::SimulationRunning,
                ..TurnFlow::default()
            })
            .id();
        world.spawn((
            simulator("fate"),
            SessionOwner(session),
            SimulationOutcome {
                turn_id: 1,
                content: "done".to_string(),
            },
        ));
        let second = world
            .spawn((
                simulator("weather"),
                SessionOwner(session),
                RunningReasoning,
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(simulator_progress_system);

        schedule.run(&mut world);
        assert_eq!(
            world.get::<TurnFlow>(session).unwrap().stage,
            TurnStage::SimulationRunning
        );

        world
            .entity_mut(second)
            .remove::<RunningReasoning>()
            .insert(SimulationOutcome {
                turn_id: 1,
                content: "done".to_string(),
            });
        schedule.run(&mut world);
        assert_eq!(
            world.get::<TurnFlow>(session).unwrap().stage,
            TurnStage::NarrationReady
        );
    }
}
