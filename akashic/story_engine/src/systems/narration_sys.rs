use agent::core::Message;
use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            Agent, AgentKind, FlowOwner, NarrationOutcome, PendingReasoning, RunningReasoning,
            SimulationOutcome,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::llm_task_manager::{TaskManager, TaskStatus},
};

// 汇总当前 flow 的模拟结果，喂给 narrator，并将其推进到叙事阶段。
pub fn collect_outcomes_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_simulators: Query<(&AgentKind, &FlowOwner, &SimulationOutcome), With<Agent>>,
    mut query_narrators: Query<
        (Entity, &mut Agent, &AgentKind, &FlowOwner),
        (Without<PendingReasoning>, Without<RunningReasoning>),
    >,
) {
    for (flow_entity, mut turn_flow) in query_flows.iter_mut() {
        if turn_flow.stage != TurnStage::CollectingOutcomes {
            continue;
        }

        let mut summaries = Vec::new();
        for (agent_kind, owner, outcome) in query_simulators.iter() {
            if owner.0 != flow_entity || *agent_kind != AgentKind::WorldSimulator {
                continue;
            }

            if outcome.turn_id == turn_flow.active_turn_id {
                summaries.push(outcome.content.clone());
            }
        }

        if summaries.is_empty() {
            continue;
        }

        let prompt = format!(
            "以下是本轮世界模拟结果，请据此生成故事叙事：\n{}",
            summaries.join("\n")
        );

        let mut dispatched_any = false;
        for (agent_entity, mut agent, agent_kind, owner) in query_narrators.iter_mut() {
            if owner.0 != flow_entity || *agent_kind != AgentKind::Narrator {
                continue;
            }

            agent.context.add_message(Message::user(&prompt));
            commands.entity(agent_entity).insert(PendingReasoning);
            dispatched_any = true;
        }

        if dispatched_any {
            turn_flow.stage = TurnStage::Narrating;
        }
    }
}

// 轮询 narrator 的运行结果，并写回叙事产物。
pub fn narration_apply_system(
    mut commands: Commands,
    mut task_manager: ResMut<TaskManager>,
    query_agents: Query<(Entity, &AgentKind, &FlowOwner), (With<Agent>, With<RunningReasoning>)>,
    query_flows: Query<&TurnFlow>,
) {
    task_manager.poll_all_tasks();

    for (agent_entity, agent_kind, owner) in query_agents.iter() {
        if *agent_kind != AgentKind::Narrator {
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
                    .insert(NarrationOutcome {
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

// narrator 完成后结束当前轮次。
pub fn narration_progress_system(
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<
        (
            &AgentKind,
            &FlowOwner,
            Option<&PendingReasoning>,
            Option<&RunningReasoning>,
            Option<&NarrationOutcome>,
        ),
        With<Agent>,
    >,
) {
    for (flow_entity, mut turn_flow) in query_flows.iter_mut() {
        if turn_flow.stage != TurnStage::Narrating {
            continue;
        }

        let mut total = 0usize;
        let mut completed = 0usize;
        let mut has_inflight = false;

        for (agent_kind, owner, pending, running, outcome) in query_agents.iter() {
            if owner.0 != flow_entity || *agent_kind != AgentKind::Narrator {
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
            turn_flow.stage = TurnStage::Ended;
        }
    }
}
