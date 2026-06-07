use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    query::{With, Without},
    system::{Commands, Query, Res, ResMut},
};

use crate::{
    components::{
        agent::{Agent, AgentOutputType, Applicator, PendingReasoning},
        flow::ApplicationCompleted,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        agent_task::{AgentTaskManager, TaskStatus},
        protagonist_action::{ProtagonistDecisionState, ProtagonistOptions},
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

#[allow(clippy::type_complexity)]
pub fn protagonist_dispatch_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow, &ProtagonistDecisionState, &WorldSnapshot)>,
    agent_tasks: Res<AgentTaskManager>,
    mut agents: Query<
        (Entity, &mut Agent, &ChildOf, Option<&ApplicationCompleted>),
        (With<Applicator>, Without<PendingReasoning>),
    >,
) {
    for (session_entity, flow, decision_state, world_snapshot) in
        sessions.iter().filter(|(_, flow, _, world_snapshot)| {
            flow.stage == TurnStage::Application && !world_snapshot.is_ending
        })
    {
        let prompt = world_snapshot.to_protagonist_prompt(Some(decision_state.committed_action()));

        for (entity, mut agent, _, _) in
            agents.iter_mut().filter(|(entity, agent, owner, outcome)| {
                owner.parent() == session_entity
                    && agent.output_type == AgentOutputType::Json
                    && agent_tasks.task_result(*entity).is_none()
                    && !outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
            })
        {
            agent.append_user_message(&prompt);
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn protagonist_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut ProtagonistDecisionState)>,
    mut agents: Query<(Entity, &mut Agent, &ChildOf), With<Applicator>>,
    mut agent_tasks: ResMut<AgentTaskManager>,
) {
    for (session_entity, mut flow, mut decision_state) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::Application)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.parent() == session_entity && agent.output_type == AgentOutputType::Json
        }) {
            let Some(result) = agent_tasks.task_result(entity).cloned() else {
                continue;
            };
            match result.status {
                TaskStatus::Done => {
                    let Some(output) = agent_tasks
                        .take_result(entity)
                        .and_then(|result| result.output)
                    else {
                        continue;
                    };
                    let Ok(options) = parse_json_response::<ProtagonistOptions>(&output) else {
                        agent.revert();
                        flow.stage = TurnStage::Failed;
                        break;
                    };
                    if options.is_empty() {
                        flow.stage = TurnStage::Failed;
                        break;
                    }
                    agent.append_assistant_message(&output);
                    decision_state.replace_with_options(options);
                    commands.entity(entity).insert(ApplicationCompleted {
                        turn_id: flow.active_turn_id,
                    });
                }
                TaskStatus::Error => {
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use agent::models::ChatModel;
    use bevy_ecs::{prelude::*, schedule::Schedule};

    use super::*;
    use crate::{components::agent::PendingReasoning, resources::agent_task::AgentTaskManager};

    #[test]
    fn skips_protagonist_dispatch_when_world_snapshot_is_ending() {
        let mut world = World::new();
        world.insert_resource(AgentTaskManager::new(ChatModel::new()));
        let session = world
            .spawn((
                TurnFlow {
                    turn_index: 6,
                    active_turn_id: 7,
                    stage: TurnStage::Application,
                },
                ProtagonistDecisionState::default(),
                WorldSnapshot {
                    is_ending: true,
                    ..WorldSnapshot::default()
                },
            ))
            .id();
        let protagonist = world
            .spawn((
                Agent::new(
                    AgentOutputType::Json,
                    "Protagonist",
                    "choose action".to_string(),
                ),
                ChildOf(session),
                Applicator,
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(protagonist_dispatch_system);

        schedule.run(&mut world);

        assert!(world.get::<PendingReasoning>(protagonist).is_none());
    }
}
