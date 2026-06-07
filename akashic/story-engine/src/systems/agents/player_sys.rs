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

#[cfg(test)]
mod tests {
    use bevy_ecs::{message::Messages, prelude::*, schedule::Schedule};

    use super::*;
    use crate::resources::protagonist_action::{
        PendingProtagonistChoice, PlayerActionInput, ProtagonistOption,
    };

    #[test]
    fn consumes_player_action_messages_for_waiting_session() {
        let mut world = World::new();
        world.insert_resource(Messages::<PlayerCommand>::default());
        let session = world
            .spawn((
                TurnFlow {
                    turn_index: 0,
                    active_turn_id: 1,
                    stage: TurnStage::AwaitingPlayer,
                },
                PlayerInputConfig::wait_for_user(),
                ProtagonistDecisionState::from_archive(
                    "start".to_string(),
                    vec![PendingProtagonistChoice {
                        id: "choice-1".to_string(),
                        option: ProtagonistOption {
                            title: "Open the door".to_string(),
                            action: "open the door".to_string(),
                            motivation_and_risk: "learn what waits outside".to_string(),
                        },
                    }],
                ),
            ))
            .id();
        world
            .resource_mut::<Messages<PlayerCommand>>()
            .write(PlayerCommand::SubmitPlayerAction {
                session_entity: session,
                turn_id: 1,
                input: PlayerActionInput {
                    r#type: PlayerActionType::SelectedOption,
                    action: "open the door".to_string(),
                },
            });
        let mut schedule = Schedule::default();
        schedule.add_systems(player_input_consume_system);

        schedule.run(&mut world);

        let decision_state = world.get::<ProtagonistDecisionState>(session).unwrap();
        assert_eq!(decision_state.committed_action(), "open the door");
        assert_eq!(
            world.get::<PlayerInputCompleted>(session),
            Some(&PlayerInputCompleted { turn_id: 1 })
        );
    }
}
