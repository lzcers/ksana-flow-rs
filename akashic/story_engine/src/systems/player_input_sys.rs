use bevy_ecs::system::Query;

use crate::{
    components::turn_flow::{TurnFlow, TurnStage},
    resources::{
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PlayerActionType, ProtagonistDecisionState},
    },
    turn_messages::PlayerCommand,
};

pub fn player_input_system(
    mut sessions: Query<(
        &mut TurnFlow,
        &PlayerInputConfig,
        &mut ProtagonistDecisionState,
        &mut PlayerInbox,
    )>,
) {
    for (mut flow, input_config, mut decision_state, mut player_inbox) in sessions.iter_mut() {
        if flow.stage != TurnStage::AwaitingPlayerChoice {
            continue;
        }

        if input_config.auto_select_first {
            let Some(action) = decision_state.first_choice_action().map(str::to_string) else {
                continue;
            };
            decision_state.commit_action(&action);
            flow.finish_turn();
            continue;
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
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{schedule::Schedule, world::World};

    use super::*;
    use crate::resources::protagonist_action::{
        PendingProtagonistChoice, PlayerActionInput, ProtagonistOption,
    };

    #[test]
    fn ignores_commands_for_another_turn() {
        let mut world = World::new();
        let mut inbox = PlayerInbox::default();
        inbox.push(PlayerCommand::SubmitPlayerAction {
            turn_id: 6,
            input: PlayerActionInput {
                r#type: PlayerActionType::SelectedOption,
                action: "continue".to_string(),
            },
        });
        let session = world
            .spawn((
                TurnFlow {
                    active_turn_id: 7,
                    stage: TurnStage::AwaitingPlayerChoice,
                    ..TurnFlow::default()
                },
                PlayerInputConfig::wait_for_user(),
                ProtagonistDecisionState::from_archive(
                    "start".to_string(),
                    vec![PendingProtagonistChoice {
                        id: "choice-1".to_string(),
                        option: ProtagonistOption {
                            title: "Continue".to_string(),
                            action: "continue".to_string(),
                            motivation_and_risk: String::new(),
                        },
                    }],
                ),
                inbox,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(player_input_system);
        schedule.run(&mut world);

        let flow = world.get::<TurnFlow>(session).unwrap();
        assert_eq!(flow.stage, TurnStage::AwaitingPlayerChoice);
        assert_eq!(
            world
                .get::<ProtagonistDecisionState>(session)
                .unwrap()
                .committed_action(),
            "start"
        );
    }

    #[test]
    fn applies_input_only_to_the_session_that_owns_the_inbox() {
        fn session_bundle(action: &str, inbox: PlayerInbox) -> impl bevy_ecs::bundle::Bundle {
            (
                TurnFlow {
                    active_turn_id: 1,
                    stage: TurnStage::AwaitingPlayerChoice,
                    ..TurnFlow::default()
                },
                PlayerInputConfig::wait_for_user(),
                ProtagonistDecisionState::from_archive(
                    "start".to_string(),
                    vec![PendingProtagonistChoice {
                        id: "choice-1".to_string(),
                        option: ProtagonistOption {
                            title: action.to_string(),
                            action: action.to_string(),
                            motivation_and_risk: String::new(),
                        },
                    }],
                ),
                inbox,
            )
        }

        let mut world = World::new();
        let mut first_inbox = PlayerInbox::default();
        first_inbox.push(PlayerCommand::SubmitPlayerAction {
            turn_id: 1,
            input: PlayerActionInput {
                r#type: PlayerActionType::SelectedOption,
                action: "first-action".to_string(),
            },
        });
        let first = world
            .spawn(session_bundle("first-action", first_inbox))
            .id();
        let second = world
            .spawn(session_bundle("second-action", PlayerInbox::default()))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(player_input_system);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<TurnFlow>(first).unwrap().stage,
            TurnStage::TurnComplete
        );
        assert_eq!(
            world.get::<TurnFlow>(second).unwrap().stage,
            TurnStage::AwaitingPlayerChoice
        );
        assert_eq!(
            world
                .get::<ProtagonistDecisionState>(second)
                .unwrap()
                .committed_action(),
            "start"
        );
    }
}
