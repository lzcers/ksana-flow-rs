use bevy_ecs::component::Component;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationCompleted {
    pub turn_id: u64,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationCompleted {
    pub turn_id: u64,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInputCompleted {
    pub turn_id: u64,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowEnd;
