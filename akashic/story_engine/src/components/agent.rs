use agent::agent::Context;
use bevy_ecs::{component::Component, entity::Entity};

#[derive(Component, Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub system_prompt: String,
    pub context: Context,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct FlowOwner(pub Entity);

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum AgentKind {
    WorldSimulator,
    Player,
    Narrator,
}

// 推理标记组件
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PendingReasoning;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RunningReasoning;
