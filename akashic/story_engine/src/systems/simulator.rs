use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query},
};

use crate::components::{
    agent::{AgentKind, FlowOwner, PendingReasoning, RunningReasoning},
    turn_flow::{TurnFlow, TurnStage},
};

// 遍历 turn_flow IDLE 下的所有 WorldSimulator Agent 实体
// 将其推动到下一模拟阶段
pub fn simulator_dispatch_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<(Entity, &AgentKind, &FlowOwner), Without<RunningReasoning>>,
) {
    for (flow_entity, mut turn_flow) in query_flows
        .iter_mut()
        .filter(|(_, turn_flow)| turn_flow.stage == TurnStage::Idle)
    {
        for (agent_entity, _, _) in query_agents.iter().filter(|&(_, agent_kind, owner)| {
            owner.0 == flow_entity && *agent_kind == AgentKind::WorldSimulator
        }) {
            commands.entity(agent_entity).insert(PendingReasoning);
        }
        turn_flow.stage.next();
    }
}

// 搜集模拟阶段的结果，并推向下一轮
pub fn simulator_collect_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    query_agents: Query<(Entity, &AgentKind, &FlowOwner), With<RunningReasoning>>,
) {
    for (flow_entity, mut turn_flow) in query_flows
        .iter_mut()
        .filter(|(_, turn_flow)| turn_flow.stage == TurnStage::SimulatingWorld)
    {
        for (agent_entity, agent_kind, _) in
            query_agents.iter().filter(|&(_, agent_kind, owner)| {
                owner.0 == flow_entity && *agent_kind == AgentKind::WorldSimulator
            })
        {}
    }
}
