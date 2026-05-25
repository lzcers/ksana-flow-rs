use bevy_ecs::{
    message::MessageWriter,
    system::{Res, ResMut},
};

use crate::{
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PlayerActionType, ProtagonistDecisionState},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{PlayerCommand, TurnEvent},
};

pub fn player_input_system(
    turn_state: Res<TurnState>,
    input_config: Res<PlayerInputConfig>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut player_inbox: ResMut<PlayerInbox>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::AwaitingPlayerChoice {
        return;
    }
    if input_config.auto_select_first {
        let Some(action) = decision_state.first_choice_action().map(str::to_string) else {
            return;
        };
        decision_state.commit_action(&action);
        event_writer.write(TurnEvent::DecisionCommitted);
        return;
    }

    while let Some(command) = player_inbox.pop() {
        match command {
            PlayerCommand::SubmitPlayerAction { turn_id, input } => {
                if turn_id != turn_state.active_turn_id {
                    continue;
                }
                let action = input.action.trim();
                if action.is_empty() {
                    continue;
                }
                if input.r#type == PlayerActionType::SelectedOption
                    && !decision_state.has_action(action)
                {
                    continue;
                }
                decision_state.commit_action(action);
                event_writer.write(TurnEvent::DecisionCommitted);
                break;
            }
        }
    }
}
