use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    system::{Query, ResMut},
};

use crate::{
    components::fate_weaver::FateWeaver,
    resources::{
        protagonist_action::ProtagonistDecisionState,
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnControl, TurnEvent},
};

// 统一管理故事轮次推进
pub fn turn_orchestrator_system(
    mut control_reader: MessageReader<TurnControl>,
    mut event_reader: MessageReader<TurnEvent>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut turn_state: ResMut<TurnState>,
    mut query: Query<(Entity, &mut FateWeaver)>,
) {
    for control in control_reader.read() {
        handle_control(control, &mut turn_state, &mut decision_state);
    }

    for event in event_reader.read() {
        handle_event(event, &mut turn_state, &mut query);
    }
}

fn handle_control(
    control: &TurnControl,
    turn_state: &mut TurnState,
    decision_state: &mut ProtagonistDecisionState,
) {
    match control {
        TurnControl::StartTurn { turn_id } => {
            turn_state.start_turn(*turn_id);
        }
        TurnControl::ContinueTurn { turn_id } => {
            turn_state.continue_turn(*turn_id);
        }
        TurnControl::ResetTurn { next_turn_id } => {
            decision_state.clear_choices();
            turn_state.reset(*next_turn_id);
        }
    }
}

// 根据各系统发送的时间，切换 turn_state
fn handle_event(
    event: &TurnEvent,
    turn_state: &mut TurnState,
    query: &mut Query<(Entity, &mut FateWeaver)>,
) {
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
        TurnEvent::PlayerChoicesReady => {
            turn_state.phase = TurnPhase::AwaitingPlayerChoice;
        }
        TurnEvent::ProtagonistCompleted => {
            turn_state.finish_turn();
        }
        TurnEvent::TaskFailed { stage, message, .. } => {
            if is_retryable_parse_failure(message) {
                match stage {
                    // 回滚到 FateWeaving phase 时，需要回滚用户输入
                    TurnPhase::FateWeaving => {
                        let Ok((_, mut fate_weaver)) = query.single_mut() else {
                            return;
                        };
                        fate_weaver.revert();
                    }
                    _ => {}
                }
                turn_state.phase = rollback_phase(*stage);
            } else {
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
        TurnPhase::AwaitingPlayerChoice => TurnPhase::AwaitingPlayerChoice,
        other => other,
    }
}
