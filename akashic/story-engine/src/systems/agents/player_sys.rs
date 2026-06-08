use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    system::{Commands, Query},
};

use crate::{
    components::{
        flow::PlayerInputCompleted,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        player_input::PlayerInputConfig,
        protagonist_action::{PlayerActionType, ProtagonistDecisionState},
    },
    turn_messages::PlayerCommand,
};

#[allow(clippy::type_complexity)]
pub fn player_input_consume_system(
    mut commands: Commands,
    mut player_commands: MessageReader<PlayerCommand>,
    mut sessions: Query<(
        Entity,
        &TurnFlow,
        &PlayerInputConfig,
        &mut ProtagonistDecisionState,
        Option<&PlayerInputCompleted>,
    )>,
) {
    let player_commands = player_commands.read().cloned().collect::<Vec<_>>();

    for (entity, flow, input_config, mut decision_state, outcome) in sessions.iter_mut() {
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

        for command in &player_commands {
            match command {
                PlayerCommand::SubmitPlayerAction {
                    session_entity,
                    turn_id,
                    input,
                } => {
                    if *session_entity != entity || *turn_id != flow.active_turn_id {
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
