use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query},
};

use crate::{
    components::{
        flow::PlayerInputCompleted,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PlayerActionType, ProtagonistDecisionState},
    },
    turn_messages::PlayerCommand,
};

#[allow(clippy::type_complexity)]
pub fn player_input_consume_system(
    mut commands: Commands,
    mut sessions: Query<(
        Entity,
        &TurnFlow,
        &PlayerInputConfig,
        &mut ProtagonistDecisionState,
        &mut PlayerInbox,
        Option<&PlayerInputCompleted>,
    )>,
) {
    for (entity, flow, input_config, mut decision_state, mut player_inbox, outcome) in
        sessions.iter_mut()
    {
        if flow.stage != TurnStage::AwaitingPlayer
            || outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
        {
            continue;
        }

        if input_config.auto_select_first {
            let Some(action) = decision_state.first_choice_action().map(str::to_string) else {
                continue;
            };
            decision_state.commit_action(&action);
            commands.entity(entity).insert(PlayerInputCompleted {
                turn_id: flow.active_turn_id,
            });
            continue;
        }

        while let Some(command) = player_inbox.pop() {
            match command {
                PlayerCommand::SubmitPlayerAction { turn_id, input } => {
                    if turn_id != flow.active_turn_id {
                        continue;
                    }
                    let action = input.action.trim();
                    if action.is_empty()
                        || (input.r#type == PlayerActionType::SelectedOption
                            && !decision_state.has_action(action))
                    {
                        continue;
                    }

                    decision_state.commit_action(action);
                    commands.entity(entity).insert(PlayerInputCompleted {
                        turn_id: flow.active_turn_id,
                    });
                    break;
                }
            }
        }
    }
}
