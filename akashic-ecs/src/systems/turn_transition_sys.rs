use bevy_ecs::{
    message::{MessageReader, MessageWriter},
    system::ResMut,
};

use crate::{
    resources::turn_state::{TurnPhase, TurnState},
    turn_messages::{TurnCommand, TurnControl, TurnEvent},
};

// 统一管理故事轮次推进，只处理 phase 与消息，不直接解析模型结果。
pub fn turn_orchestrator_system(
    mut control_reader: MessageReader<TurnControl>,
    mut event_reader: MessageReader<TurnEvent>,
    mut command_writer: MessageWriter<TurnCommand>,
    mut turn_state: ResMut<TurnState>,
) {
    let mut should_start_turn = turn_state.phase == TurnPhase::Idle;

    for control in control_reader.read() {
        match control {
            TurnControl::StartTurn { turn_id } if turn_state.phase == TurnPhase::Idle => {
                turn_state.active_turn_id = *turn_id;
                should_start_turn = true;
            }
            TurnControl::StartTurn { .. } => {}
            TurnControl::ResetTurn { next_turn_id } => {
                turn_state.reset(*next_turn_id);
                should_start_turn = true;
            }
        }
    }

    for event in event_reader.read() {
        match event {
            TurnEvent::FateWeavingCompleted {
                turn_id,
                requires_protagonist,
                has_choices,
                summary: _summary,
                fate_summary,
            } if *turn_id == turn_state.active_turn_id => {
                command_writer.write(TurnCommand::SyncFateContext {
                    turn_id: *turn_id,
                    summary: fate_summary.clone(),
                });
                if *requires_protagonist || *has_choices {
                    turn_state.phase = TurnPhase::AwaitingProtagonistResult;
                    command_writer.write(TurnCommand::RequestProtagonist { turn_id: *turn_id });
                } else {
                    turn_state.phase = TurnPhase::RoundCompleted;
                    command_writer.write(TurnCommand::RequestNarration { turn_id: *turn_id });
                    turn_state.phase = TurnPhase::AwaitingNarrationResult;
                }
            }
            TurnEvent::ProtagonistDecided {
                turn_id,
                summary: _summary,
            } if *turn_id == turn_state.active_turn_id => {
                turn_state.phase = TurnPhase::RoundCompleted;
                command_writer.write(TurnCommand::RequestNarration { turn_id: *turn_id });
                turn_state.phase = TurnPhase::AwaitingNarrationResult;
            }
            TurnEvent::NarrationCompleted {
                turn_id,
                summary: _summary,
            } if *turn_id == turn_state.active_turn_id => {
                turn_state.finish_turn();
                should_start_turn = true;
            }
            TurnEvent::TaskFailed {
                turn_id,
                stage: _stage,
                entity: _entity,
                message: _message,
            } if *turn_id == turn_state.active_turn_id => {
                turn_state.phase = TurnPhase::Failed;
            }
            _ => {}
        }
    }

    if should_start_turn && turn_state.phase == TurnPhase::Idle {
        let turn_id = turn_state.active_turn_id;
        turn_state.start_turn(turn_id);
        command_writer.write(TurnCommand::RequestFate { turn_id });
    }
}
