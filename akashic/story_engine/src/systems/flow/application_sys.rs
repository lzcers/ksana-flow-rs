use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            Agent, AgentOutputKind, NarrationOutcome, OwnedAgentMut, PendingReasoning,
            PipelinePhase, ReadyAgentFilter, RunningReasoning, SessionOwner, SimulationOutcome,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::{ProtagonistDecisionState, ProtagonistOptions},
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

pub fn application_dispatch_system(
    mut commands: Commands,
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &ProtagonistDecisionState,
        &WorldSnapshot,
    )>,
    simulation_outcomes: Query<(&SessionOwner, &SimulationOutcome)>,
    mut agents: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    for (session_entity, mut flow, decision_state, world_snapshot) in sessions
        .iter_mut()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::ApplicationReady)
    {
        let simulation_results = simulation_outcomes
            .iter()
            .filter(|(owner, outcome)| {
                owner.0 == session_entity && outcome.turn_id == flow.active_turn_id
            })
            .map(|(_, outcome)| outcome.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let narration_prompt = format!(
            "{}\n\n【模拟阶段结果】\n{}",
            world_snapshot.to_story_prompt(Some(decision_state.committed_action())),
            simulation_results
        );
        let protagonist_prompt =
            world_snapshot.to_protagonist_prompt(Some(decision_state.committed_action()));

        let mut dispatched = 0;
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.output_kind.pipeline_phase() == PipelinePhase::Application
                && (!world_snapshot.is_ending
                    || agent.output_kind != AgentOutputKind::ProtagonistOptions)
        }) {
            match agent.output_kind {
                AgentOutputKind::Narration => agent.append_user_message(&narration_prompt),
                AgentOutputKind::ProtagonistOptions => {
                    agent.append_user_message(&protagonist_prompt)
                }
                AgentOutputKind::WorldSnapshot | AgentOutputKind::SimulationText => continue,
            }
            commands.entity(entity).insert(PendingReasoning);
            dispatched += 1;
        }

        flow.stage = if dispatched > 0 {
            TurnStage::ApplicationRunning
        } else {
            TurnStage::Failed
        };
    }
}

pub fn application_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut ProtagonistDecisionState)>,
    mut agents: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow, mut decision_state) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::ApplicationRunning)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.output_kind.pipeline_phase() == PipelinePhase::Application
        }) {
            let Some(result) = task_manager.task_result(entity).cloned() else {
                continue;
            };
            match result.status {
                TaskStatus::Done => {
                    let Some(output) = task_manager
                        .take_result(entity)
                        .and_then(|result| result.output)
                    else {
                        continue;
                    };
                    match agent.output_kind {
                        AgentOutputKind::Narration => {
                            agent.append_assistant_message(&output);
                            commands.entity(entity).remove::<RunningReasoning>().insert(
                                NarrationOutcome {
                                    turn_id: flow.active_turn_id,
                                    content: output,
                                },
                            );
                        }
                        AgentOutputKind::ProtagonistOptions => {
                            let Ok(options) = parse_json_response::<ProtagonistOptions>(&output)
                            else {
                                agent.revert();
                                commands.entity(entity).remove::<RunningReasoning>();
                                flow.stage = TurnStage::Failed;
                                break;
                            };
                            if options.is_empty() {
                                commands.entity(entity).remove::<RunningReasoning>();
                                flow.stage = TurnStage::Failed;
                                break;
                            }
                            agent.append_assistant_message(&output);
                            decision_state.replace_with_options(options);
                            commands.entity(entity).remove::<RunningReasoning>();
                        }
                        AgentOutputKind::WorldSnapshot | AgentOutputKind::SimulationText => {}
                    }
                }
                TaskStatus::Error => {
                    task_manager.clear_task(entity);
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn application_progress_system(
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &WorldSnapshot,
        &ProtagonistDecisionState,
    )>,
    agents: Query<(
        &Agent,
        &SessionOwner,
        Option<&PendingReasoning>,
        Option<&RunningReasoning>,
        Option<&NarrationOutcome>,
    )>,
) {
    for (session_entity, mut flow, world_snapshot, decision_state) in sessions
        .iter_mut()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::ApplicationRunning)
    {
        let mut application_count = 0;
        let mut completed_count = 0;
        let mut has_inflight = false;
        for (agent, _, pending, running, narration) in agents.iter().filter(|(agent, owner, ..)| {
            owner.0 == session_entity
                && agent.output_kind.pipeline_phase() == PipelinePhase::Application
                && (!world_snapshot.is_ending
                    || agent.output_kind != AgentOutputKind::ProtagonistOptions)
        }) {
            application_count += 1;
            has_inflight |= pending.is_some() || running.is_some();
            completed_count += usize::from(match agent.output_kind {
                AgentOutputKind::Narration => {
                    narration.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
                }
                AgentOutputKind::ProtagonistOptions => !decision_state.choices().is_empty(),
                AgentOutputKind::WorldSnapshot | AgentOutputKind::SimulationText => false,
            });
        }

        if flow.stage == TurnStage::Failed || application_count == 0 || has_inflight {
            continue;
        }
        if completed_count == application_count {
            if world_snapshot.is_ending {
                flow.finish_story();
            } else {
                flow.stage = TurnStage::AwaitingPlayerChoice;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use agent::agent::Context;
    use bevy_ecs::{schedule::Schedule, world::World};

    use super::*;
    use crate::resources::protagonist_action::{PendingProtagonistChoice, ProtagonistOption};

    fn agent(output_kind: AgentOutputKind, name: &str) -> Agent {
        Agent::from_context(output_kind, name, Context::default())
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
            agent(AgentOutputKind::Narration, "Narrator"),
            SessionOwner(session),
            NarrationOutcome {
                turn_id: 1,
                content: "story".to_string(),
            },
        ));
        let protagonist = world
            .spawn((
                agent(AgentOutputKind::ProtagonistOptions, "Protagonist"),
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
            .replace_with_options(ProtagonistOptions {
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
            agent(AgentOutputKind::Narration, "Narrator"),
            SessionOwner(session),
            NarrationOutcome {
                turn_id: 1,
                content: "ending".to_string(),
            },
        ));
        world.spawn((
            agent(AgentOutputKind::ProtagonistOptions, "Protagonist"),
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
