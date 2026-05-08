use bevy_ecs::{message::MessageReader, system::ResMut};

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
        TurnEvent::TaskFailed { stage, message, .. } => {
            if is_retryable_parse_failure(message) {
                println!("JSON 解析失败，回滚到阶段: {:?}", stage);
                turn_state.phase = rollback_phase(*stage);
            } else {
                println!("任务失败，错误信息: {:?}", message);
                turn_state.phase = TurnPhase::Failed;
            }
        }
    }
}

fn is_retryable_parse_failure(message: &str) -> bool {
    message.contains("无法解析 JSON 响应")
}

fn rollback_phase(stage: TurnPhase) -> TurnPhase {
    match stage {
        TurnPhase::FateWeaving => TurnPhase::Idle,
        TurnPhase::AwaitingProtagonist => TurnPhase::ProtagonistAction,
        other => other,
    }
}
