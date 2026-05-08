use bevy_ecs::{entity::Entity, message::Message};

use crate::resources::turn_state::TurnPhase;

#[derive(Message, Debug, Clone)]
pub enum TurnControl {
    StartTurn { turn_id: u64 },
    ResetTurn { next_turn_id: Option<u64> },
    SubmitProtagonistAction { turn_id: u64, action_text: String },
}

#[derive(Message, Debug, Clone)]
pub enum TurnEvent {
    FatePlanning,         // 命运编织状态
    FateGenerated,        // 命运编织完成
    StoryWriting,         // 故事编写中
    StoryGenerated,       // 故事编排完成
    AwaitingProtagonist,  // 等待主角状态
    ProtagonistCompleted, // 主角完成状态
    TaskFailed {
        turn_id: u64,
        stage: TurnPhase,
        entity: Entity,
        message: String,
    },
}
