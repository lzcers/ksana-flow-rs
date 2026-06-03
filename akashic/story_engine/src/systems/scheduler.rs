use crate::{
    components::agent::{Agent, PendingReasoning, RunningReasoning},
    resources::llm_task_manager::TaskManager,
};
use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Query, ResMut},
};

// 调度系统
// 主要工作是扫描所有的 Agent Entity，根据当前阶段和 Agent 状态，将需要调度的 Agent 加入到任务队列中
pub fn agent_scheduler_system(
    query: Query<(Entity, &Agent), With<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (entity, agent) in query.iter() {}
}
