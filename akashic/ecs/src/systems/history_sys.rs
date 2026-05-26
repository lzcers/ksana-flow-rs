use agent::core::Message;
use bevy_ecs::{
    message::MessageReader,
    system::{Query, Res, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::{
        history::SessionHistoryLog, protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
    turn_messages::TurnEvent,
};

pub fn history_sys(
    mut event_reader: MessageReader<TurnEvent>,
    mut history: ResMut<SessionHistoryLog>,
    world_snapshot: Res<WorldSnapshot>,
    decision_state: Res<ProtagonistDecisionState>,
    narrator_query: Query<&UpperNarrator>,
) {
    for event in event_reader.read() {
        let round = world_snapshot.round.max(1);
        match event {
            TurnEvent::FateCompleted => {
                history.set_world_snapshot(round, world_snapshot.clone());
            }
            TurnEvent::NarrationCompleted => {
                let Ok(narrator) = narrator_query.single() else {
                    continue;
                };
                let Some(text) = latest_assistant_message(narrator.context().conversation().as_slice())
                else {
                    continue;
                };
                history.set_narration(round, text);
            }
            TurnEvent::ChoicesPrepared => {
                history.set_choices(round, decision_state.choices().to_vec());
            }
            TurnEvent::DecisionCommitted => {
                let action = decision_state.committed_action().trim();
                if action.is_empty() || action == "start" {
                    continue;
                }
                history.set_committed_action(round, action.to_string());
            }
            TurnEvent::FateStarted
            | TurnEvent::NarrationStarted
            | TurnEvent::ProtagonistStarted
            | TurnEvent::TaskFailed { .. } => {}
        }
    }
}

fn latest_assistant_message(messages: &[Message]) -> Option<String> {
    messages.iter().rev().find_map(|message| match message {
        Message::Assistant { content, .. } if !content.trim().is_empty() => {
            Some(content.trim().to_string())
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use agent::{agent::Context, core::Message};
    use bevy_ecs::{
        message::{Messages, message_update_system},
        prelude::*,
        schedule::Schedule,
    };

    use super::*;
    use crate::{
        components::upper_narrator::UpperNarrator,
        resources::protagonist_action::{PendingProtagonistChoice, ProtagonistOption},
    };

    #[test]
    fn history_sys_records_round_facts_from_turn_events() {
        let mut world = World::new();
        world.init_resource::<Messages<TurnEvent>>();
        world.init_resource::<SessionHistoryLog>();
        world.insert_resource(WorldSnapshot {
            round: 3,
            scene_title: "雨夜塔楼".to_string(),
            ..WorldSnapshot::default()
        });
        world.insert_resource(ProtagonistDecisionState::from_archive(
            "沿屋檐潜行".to_string(),
            vec![PendingProtagonistChoice {
                id: "choice-1".to_string(),
                option: ProtagonistOption {
                    title: "潜行".to_string(),
                    action: "沿屋檐潜行".to_string(),
                    motivation_and_risk: "更隐蔽，但失足就会坠落".to_string(),
                },
            }],
        ));
        let mut narrator_context = Context::default();
        narrator_context.add_message(Message::assistant("雨水拍打着铜钟外壳。"));
        world.spawn(UpperNarrator::from_context(narrator_context));

        {
            let mut events = world.resource_mut::<Messages<TurnEvent>>();
            events.write(TurnEvent::FateCompleted);
            events.write(TurnEvent::NarrationCompleted);
            events.write(TurnEvent::ChoicesPrepared);
            events.write(TurnEvent::DecisionCommitted);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems((message_update_system, history_sys).chain());
        schedule.run(&mut world);

        let history = world.resource::<SessionHistoryLog>();
        assert_eq!(history.rounds.len(), 1);
        let entry = &history.rounds[0];
        assert_eq!(entry.round, 3);
        assert_eq!(
            entry.world_snapshot.as_ref().map(|snapshot| snapshot.scene_title.as_str()),
            Some("雨夜塔楼")
        );
        assert_eq!(entry.narration_text.as_deref(), Some("雨水拍打着铜钟外壳。"));
        assert_eq!(entry.choices.len(), 1);
        assert_eq!(entry.committed_action.as_deref(), Some("沿屋檐潜行"));
    }
}
