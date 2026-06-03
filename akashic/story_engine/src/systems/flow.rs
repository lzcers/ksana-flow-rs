use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query},
};

use crate::components::{
    agent::{AgentKind, FlowOwner, PendingReasoning, RunningReasoning},
    turn_flow::{TurnFlow, TurnStage},
};

// 查询所有不在推理中的 Agent 实体，判断哪些实体需要在当前阶段提交给调度器进入推理状态
pub fn flow_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<(Entity, &AgentKind, &FlowOwner), Without<RunningReasoning>>,
) {
    // 遍历当前 turn_flow 下的所有 Agent 实体
    for (flow_entity, mut turn_flow) in query_flows.iter_mut() {
        for (agent_entity, agent_kind, _) in query_agents
            .iter()
            .filter(|(_, _, owner)| owner.0 == flow_entity)
        {
            // todo: 这里应该做成可配置的
            match turn_flow.stage {
                TurnStage::Idle => {
                    // 将所有 WorldSimulator 进入推理状态，等待调度器调度
                    if *agent_kind == AgentKind::WorldSimulator {
                        commands.entity(agent_entity).insert(PendingReasoning);
                        turn_flow.stage.next();
                    }
                }
                TurnStage::CollectingOutcomes => {
                    // 将所有 Player 或 Narrator 进入推理状态，等待调度器调度
                    if *agent_kind == AgentKind::Player || *agent_kind == AgentKind::Narrator {
                        commands.entity(agent_entity).insert(PendingReasoning);
                        turn_flow.stage.next();
                    }
                }
                _ => continue,
            }
        }
    }
}
