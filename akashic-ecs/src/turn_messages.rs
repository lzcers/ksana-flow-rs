use bevy_ecs::{
    entity::Entity,
    message::{Message, MessageRegistry},
    world::World,
};

use crate::resources::turn_state::TurnPhase;

#[derive(Message, Debug, Clone)]
pub enum TurnControl {
    StartTurn { turn_id: u64 },
    ResetTurn { next_turn_id: Option<u64> },
}

#[derive(Message, Debug, Clone)]
pub enum TurnCommand {
    RequestFate { turn_id: u64 },
    RequestProtagonist { turn_id: u64 },
    RequestNarration { turn_id: u64 },
}

#[derive(Message, Debug, Clone)]
pub enum TurnEvent {
    FateWeavingCompleted {
        turn_id: u64,
        requires_protagonist: bool,
        has_choices: bool,
        summary: String,
    },
    ProtagonistDecided {
        turn_id: u64,
        summary: String,
    },
    NarrationCompleted {
        turn_id: u64,
        summary: String,
    },
    TaskFailed {
        turn_id: u64,
        stage: TurnPhase,
        entity: Entity,
        message: String,
    },
}

pub fn register_turn_messages(world: &mut World) {
    MessageRegistry::register_message::<TurnControl>(world);
    MessageRegistry::register_message::<TurnCommand>(world);
    MessageRegistry::register_message::<TurnEvent>(world);
}
