use bevy_ecs::component::Component;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct NarrationOutcome {
    pub turn_id: u64,
    pub content: String,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SimulationOutcome {
    pub turn_id: u64,
    pub content: String,
}
