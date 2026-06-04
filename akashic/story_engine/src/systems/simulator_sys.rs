use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            Agent, AgentKind, FlowOwner, PendingReasoning, RunningReasoning, SimulationOutcome,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::llm_task_manager::{TaskManager, TaskStatus},
};

// 遍历 turn_flow IDLE 下的所有 WorldSimulator Agent 实体，
// 为其打上待调度标记并推进到模拟阶段。
pub fn simulator_dispatch_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<
        (Entity, &AgentKind, &FlowOwner),
        (Without<PendingReasoning>, Without<RunningReasoning>),
    >,
) {
    for (flow_entity, mut turn_flow) in query_flows.iter_mut() {
        if turn_flow.stage != TurnStage::Idle {
            continue;
        }

        let mut dispatched_any = false;

        for (agent_entity, agent_kind, owner) in query_agents.iter() {
            if owner.0 != flow_entity || *agent_kind != AgentKind::WorldSimulator {
                continue;
            }

            commands.entity(agent_entity).insert(PendingReasoning);
            dispatched_any = true;
        }

        if dispatched_any {
            turn_flow.stage = TurnStage::SimulatingWorld;
        }
    }
}

// 轮询已经进入运行中的模拟任务，并将完成结果写回实体。
pub fn simulator_apply_system(
    mut commands: Commands,
    mut task_manager: ResMut<TaskManager>,
    query_agents: Query<(Entity, &AgentKind, &FlowOwner), (With<Agent>, With<RunningReasoning>)>,
    query_flows: Query<&TurnFlow>,
) {
    task_manager.poll_all_tasks();

    for (agent_entity, agent_kind, owner) in query_agents.iter() {
        if *agent_kind != AgentKind::WorldSimulator {
            continue;
        }

        let Ok(flow) = query_flows.get(owner.0) else {
            continue;
        };

        match task_manager.poll_task(agent_entity) {
            TaskStatus::Done => {
                let Some(result) = task_manager.take_result(agent_entity) else {
                    continue;
                };

                let Some(output) = result.output else {
                    continue;
                };

                commands
                    .entity(agent_entity)
                    .remove::<RunningReasoning>()
                    .insert(SimulationOutcome {
                        turn_id: flow.active_turn_id,
                        content: output,
                    });
            }
            TaskStatus::Error => {
                commands.entity(agent_entity).remove::<RunningReasoning>();
                task_manager.clear_task(agent_entity);
            }
            TaskStatus::Pending | TaskStatus::Running => {}
        }
    }
}

// 当前 flow 下的模拟实体都完成后，再推进到结果汇总阶段。
pub fn simulator_progress_system(
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<
        (
            &AgentKind,
            &FlowOwner,
            Option<&PendingReasoning>,
            Option<&RunningReasoning>,
            Option<&SimulationOutcome>,
        ),
        With<Agent>,
    >,
) {
    for (flow_entity, mut turn_flow) in query_flows.iter_mut() {
        if turn_flow.stage != TurnStage::SimulatingWorld {
            continue;
        }

        let mut total = 0usize;
        let mut completed = 0usize;
        let mut has_inflight = false;

        for (agent_kind, owner, pending, running, outcome) in query_agents.iter() {
            if owner.0 != flow_entity || *agent_kind != AgentKind::WorldSimulator {
                continue;
            }

            total += 1;

            if pending.is_some() || running.is_some() {
                has_inflight = true;
            }

            if let Some(outcome) = outcome
                && outcome.turn_id == turn_flow.active_turn_id
            {
                completed += 1;
            }
        }

        if total > 0 && !has_inflight && completed == total {
            turn_flow.stage = TurnStage::CollectingOutcomes;
        }
    }
}
