use bevy_ecs::{entity::Entity, system::Query};

use crate::{
    components::{
        agent::{
            Agent, AgentOutputType, NarrationOutcome, PendingReasoning, PipelinePhase,
            RunningReasoning, SessionOwner,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{protagonist_action::ProtagonistDecisionState, world_snapshot::WorldSnapshot},
};

type ApplicationProgressAgent<'a> = (
    &'a Agent,
    &'a SessionOwner,
    Option<&'a PendingReasoning>,
    Option<&'a RunningReasoning>,
    Option<&'a NarrationOutcome>,
);

pub fn application_progress_system(
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &WorldSnapshot,
        &ProtagonistDecisionState,
    )>,
    agents: Query<ApplicationProgressAgent>,
) {
    for (session_entity, mut flow, world_snapshot, decision_state) in
        sessions.iter_mut().filter(|(_, flow, ..)| {
            matches!(
                flow.stage,
                TurnStage::ApplicationReady | TurnStage::ApplicationRunning
            )
        })
    {
        let mut application_count = 0;
        let mut completed_count = 0;
        let mut has_inflight = false;
        for (agent, _, pending, running, narration) in agents.iter().filter(|(agent, owner, ..)| {
            owner.0 == session_entity
                && agent.kind.pipeline_phase() == PipelinePhase::Application
                && (!world_snapshot.is_ending || agent.output_type != AgentOutputType::Json)
        }) {
            application_count += 1;
            has_inflight |= pending.is_some() || running.is_some();
            completed_count += usize::from(match agent.output_type {
                AgentOutputType::Text => {
                    narration.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
                }
                AgentOutputType::Json => !decision_state.choices().is_empty(),
            });
        }

        if application_count == 0 {
            flow.stage = TurnStage::Failed;
            continue;
        }
        if has_inflight || completed_count < application_count {
            flow.stage = TurnStage::ApplicationRunning;
            continue;
        }
        if world_snapshot.is_ending {
            flow.finish_story();
        } else {
            flow.stage = TurnStage::AwaitingPlayerChoice;
        }
    }
}

#[cfg(test)]
mod tests {
    use agent::agent::Context;
    use bevy_ecs::{schedule::Schedule, world::World};

    use super::*;
    use crate::{
        components::agent::{AgentKind, AgentOutputType},
        resources::protagonist_action::{PendingProtagonistChoice, ProtagonistOption},
    };

    fn agent(output_type: AgentOutputType, name: &str) -> Agent {
        Agent::from_context(
            AgentKind::Applicator,
            output_type,
            name,
            String::new(),
            Context::default(),
        )
    }

    fn choice(action: &str) -> PendingProtagonistChoice {
        PendingProtagonistChoice {
            id: format!("choice-{action}"),
            option: ProtagonistOption {
                title: action.to_string(),
                action: action.to_string(),
                motivation_and_risk: String::new(),
            },
        }
    }

    #[test]
    fn application_progress_waits_for_story_and_options() {
        let mut world = World::new();
        let session = world
            .spawn((
                TurnFlow {
                    active_turn_id: 1,
                    stage: TurnStage::ApplicationRunning,
                    ..TurnFlow::default()
                },
                WorldSnapshot::default(),
                ProtagonistDecisionState::default(),
            ))
            .id();
        world.spawn((
            agent(AgentOutputType::Text, "Narrator"),
            SessionOwner(session),
            NarrationOutcome {
                turn_id: 1,
                content: "story".to_string(),
            },
        ));
        let protagonist = world
            .spawn((
                agent(AgentOutputType::Json, "Protagonist"),
                SessionOwner(session),
                RunningReasoning,
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(application_progress_system);

        schedule.run(&mut world);
        assert_eq!(
            world.get::<TurnFlow>(session).unwrap().stage,
            TurnStage::ApplicationRunning
        );

        world.entity_mut(protagonist).remove::<RunningReasoning>();
        world
            .get_mut::<ProtagonistDecisionState>(session)
            .unwrap()
            .replace_with_options(crate::resources::protagonist_action::ProtagonistOptions {
                options: vec![choice("continue").option],
            });
        schedule.run(&mut world);
        assert_eq!(
            world.get::<TurnFlow>(session).unwrap().stage,
            TurnStage::AwaitingPlayerChoice
        );
    }

    #[test]
    fn application_progress_finishes_story_without_protagonist_when_ending() {
        let mut world = World::new();
        let session = world
            .spawn((
                TurnFlow {
                    active_turn_id: 1,
                    stage: TurnStage::ApplicationRunning,
                    ..TurnFlow::default()
                },
                WorldSnapshot {
                    is_ending: true,
                    ..WorldSnapshot::default()
                },
                ProtagonistDecisionState::default(),
            ))
            .id();
        world.spawn((
            agent(AgentOutputType::Text, "Narrator"),
            SessionOwner(session),
            NarrationOutcome {
                turn_id: 1,
                content: "ending".to_string(),
            },
        ));
        world.spawn((
            agent(AgentOutputType::Json, "Protagonist"),
            SessionOwner(session),
        ));
        let mut schedule = Schedule::default();
        schedule.add_systems(application_progress_system);

        schedule.run(&mut world);

        assert_eq!(
            world.get::<TurnFlow>(session).unwrap().stage,
            TurnStage::StoryEnded
        );
    }
}
