use bevy_ecs::component::Component;

#[derive(Component, Debug)]
pub struct TurnFlow {
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub stage: TurnStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStage {
    Idle,
    AwaitingPlayerInput,
    SimulatingWorld,
    CollectingOutcomes,
    NarrationReady,
    Narrating,
    AwaitingNextTurn,
    Ended,
    Failed,
}
