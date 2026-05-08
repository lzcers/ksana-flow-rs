use bevy_ecs::{message::MessageReader, system::ResMut};
use uuid::fmt::Urn;

use crate::{
    resources::turn_state::{TurnPhase, TurnState},
    turn_messages::TurnEvent,
};

// 统一管理故事轮次推进
pub fn turn_orchestrator_system(
    mut event_reader: MessageReader<TurnEvent>,
    mut turn_state: ResMut<TurnState>,
) {
    for event in event_reader.read() {
        handle_event(event, &mut turn_state);
    }
}

// 根据各系统发送的时间，切换 turn_state
fn handle_event(event: &TurnEvent, turn_state: &mut TurnState) {
    match event {
        TurnEvent::FatePlanning => {
            turn_state.phase = TurnPhase::FateWeaving;
        }
        TurnEvent::FateGenerated => {
            turn_state.phase = TurnPhase::NarratorStory;
        }
        TurnEvent::StoryWriting => {
            turn_state.phase = TurnPhase::NarratorWriting;
        }
        TurnEvent::StoryGenerated => {
            turn_state.phase = TurnPhase::ProtagonistAction;
        }
        TurnEvent::AwaitingProtagonist => {
            turn_state.phase = TurnPhase::AwaitingProtagonist;
        }
        TurnEvent::ProtagonistCompleted => {
            turn_state.finish_turn();
        }
        TurnEvent::TaskFailed { .. } => {
            turn_state.phase = TurnPhase::Failed;
        }
        _ => {}
    }
}
