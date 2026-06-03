use agent::agent::Context;
use bevy_ecs::component::Component;

#[derive(Component, Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub system_prompt: String,
    pub context: Context,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Running,
}
