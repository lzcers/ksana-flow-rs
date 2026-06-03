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
    SimulatingWorld,
    CollectingOutcomes,
    Narrating,
    Ended,
    Failed,
}

impl TurnStage {
    pub fn next(&mut self) {
        match *self {
            TurnStage::Idle => *self = TurnStage::SimulatingWorld,
            TurnStage::SimulatingWorld => *self = TurnStage::CollectingOutcomes,
            TurnStage::CollectingOutcomes => *self = TurnStage::Narrating,
            TurnStage::Narrating => *self = TurnStage::Ended,
            TurnStage::Ended => *self = TurnStage::Failed,
            TurnStage::Failed => *self = TurnStage::Idle,
        }
    }
}
