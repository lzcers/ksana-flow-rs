use bevy_ecs::{entity::Entity, resource::Resource};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Idle,
    AwaitingFateResult,
    AwaitingProtagonist,
    AwaitingNarration,
    RoundCompleted,
}

impl Default for TurnPhase {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Resource, Debug, Default)]
pub struct TurnState {
    pub phase: TurnPhase,
    pub active_fate_weaver: Option<Entity>,
}
