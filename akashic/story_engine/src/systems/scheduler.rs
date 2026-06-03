use crate::components::agent::{Agent, State};
use bevy_ecs::system::Query;

// 调度系统
// 主要工作是扫描所有的 Agent Entity，根据当前阶段和 Agent 状态，将需要调度的 Agent 加入到任务队列中
pub fn agent_scheduler(mut query: Query<(&mut Agent, &mut State)>) {
    for (agent, state) in query.iter_mut() {}
}
