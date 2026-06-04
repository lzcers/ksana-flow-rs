use bevy_ecs::system::{Query, Res, ResMut};

use crate::{
    components::turn_flow::{TurnFlow, TurnStage},
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PlayerActionType, ProtagonistDecisionState},
    },
    turn_messages::PlayerCommand,
};

pub fn player_input_system(
    mut query_flows: Query<&mut TurnFlow>,
    input_config: Res<PlayerInputConfig>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut player_inbox: ResMut<PlayerInbox>,
) {
    let Ok(mut flow) = query_flows.single_mut() else {
        return;
    };
    if flow.stage != TurnStage::AwaitingPlayerChoice {
        return;
    }

    if input_config.auto_select_first {
        let Some(action) = decision_state.first_choice_action().map(str::to_string) else {
            return;
        };
        decision_state.commit_action(&action);
        flow.finish_turn();
        return;
    }

    while let Some(command) = player_inbox.pop() {
        match command {
            PlayerCommand::SubmitPlayerAction { turn_id, input } => {
                if turn_id != flow.active_turn_id {
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
                flow.finish_turn();
                break;
            }
        }
    }
}
