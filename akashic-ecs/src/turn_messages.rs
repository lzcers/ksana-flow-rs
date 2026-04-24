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
    SubmitProtagonistAction { turn_id: u64, action_text: String },
}

#[derive(Message, Debug, Clone)]
pub enum TurnEvent {
    SceneFateGenerated {
        turn_id: u64,
        scene_facts: String,
    },
    SceneNarrationGenerated {
        turn_id: u64,
        scene_text: String,
    },
    ProtagonistActionGenerated {
        turn_id: u64,
        action_text: String,
    },
    ConsequenceFateGenerated {
        turn_id: u64,
        consequence_facts: String,
    },
    StoryNarrationGenerated {
        turn_id: u64,
        story_text: String,
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
    MessageRegistry::register_message::<TurnEvent>(world);
}
