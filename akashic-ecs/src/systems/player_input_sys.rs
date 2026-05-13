use bevy_ecs::{
    message::MessageWriter,
    system::{Res, ResMut},
};

use crate::{
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::ProtagonistDecisionState,
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
        let Some(choice_id) = decision_state.first_choice_id().map(str::to_string) else {
            return;
        };
        decision_state.commit_selection(&choice_id);
        event_writer.write(TurnEvent::ProtagonistCompleted);
        return;
    }

    while let Some(command) = player_inbox.pop() {
        match command {
            PlayerCommand::SubmitChoice { turn_id, choice_id } => {
                if turn_id != turn_state.active_turn_id {
                    continue;
                }
                if decision_state.find_action(&choice_id).is_none() {
                    continue;
                }
                decision_state.commit_selection(&choice_id);
                event_writer.write(TurnEvent::ProtagonistCompleted);
                break;
            }
        }
    }
}
