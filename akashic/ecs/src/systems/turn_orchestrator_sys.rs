use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    system::{Query, Res, ResMut},
};

use crate::{
    components::fate_weaver::FateWeaver,
    resources::{
        protagonist_action::ProtagonistDecisionState,
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    turn_messages::{TurnControl, TurnEvent},
};

// 统一管理故事轮次推进
pub fn turn_orchestrator_system(
    mut control_reader: MessageReader<TurnControl>,
    mut event_reader: MessageReader<TurnEvent>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut turn_state: ResMut<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    mut query: Query<(Entity, &mut FateWeaver)>,
) {
    for control in control_reader.read() {
        handle_control(control, &mut turn_state, &mut decision_state);
    }

    for event in event_reader.read() {
        handle_event(
            event,
            &mut turn_state,
            &world_snapshot,
            &mut decision_state,
            &mut query,
        );
    }
}

fn handle_control(
    control: &TurnControl,
    turn_state: &mut TurnState,
    decision_state: &mut ProtagonistDecisionState,
) {
    match control {
        TurnControl::StartNextTurn => {
            turn_state.advance();
        }
        TurnControl::ResetToIdle { next_turn_id } => {
            decision_state.clear_choices();
            turn_state.reset(*next_turn_id);
        }
    }
}

// 根据各系统发送的事件，切换 turn_state
fn handle_event(
    event: &TurnEvent,
    turn_state: &mut TurnState,
    world_snapshot: &WorldSnapshot,
    decision_state: &mut ProtagonistDecisionState,
    query: &mut Query<(Entity, &mut FateWeaver)>,
) {
    match event {
        TurnEvent::FateStarted => {
            turn_state.phase = TurnPhase::FateRunning;
        }
        TurnEvent::FateCompleted => {
            turn_state.phase = TurnPhase::NarrationReady;
        }
        TurnEvent::NarrationStarted => {
            turn_state.phase = TurnPhase::NarrationRunning;
        }
        TurnEvent::NarrationCompleted => {
            if world_snapshot.is_ending {
                decision_state.clear_choices();
                turn_state.finish_story();
            } else {
                turn_state.phase = TurnPhase::ProtagonistReady;
            }
        }
        TurnEvent::ProtagonistStarted => {
            turn_state.phase = TurnPhase::ProtagonistRunning;
        }
        TurnEvent::ChoicesPrepared => {
            turn_state.phase = TurnPhase::AwaitingPlayerChoice;
        }
        TurnEvent::DecisionCommitted => {
            turn_state.finish_turn();
        }
        TurnEvent::TaskFailed { stage, message, .. } => {
            if is_retryable_parse_failure(message) {
                match stage {
                    // Fate 重试前需要回滚上一轮追加的输入
                    TurnPhase::FateRunning => {
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
        TurnPhase::FateRunning => TurnPhase::TurnReady,
        TurnPhase::ProtagonistRunning => TurnPhase::ProtagonistReady,
        TurnPhase::AwaitingPlayerChoice => TurnPhase::AwaitingPlayerChoice,
        other => other,
    }
}
