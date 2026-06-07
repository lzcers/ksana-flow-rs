use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            Agent, AgentOutputType, Applicator, PendingReasoning, RunningReasoning, SessionOwner,
        },
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
    mut agents: Query<
        (
            Entity,
            &mut Agent,
            &SessionOwner,
            Option<&ApplicationCompleted>,
        ),
        (
            With<Applicator>,
            Without<PendingReasoning>,
            Without<RunningReasoning>,
        ),
    >,
) {
    for (session_entity, flow, decision_state, world_snapshot) in sessions
        .iter()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::Application)
    {
        let prompt = world_snapshot.to_protagonist_prompt(Some(decision_state.committed_action()));

        for (entity, mut agent, _, _) in agents.iter_mut().filter(|(_, agent, owner, outcome)| {
            owner.0 == session_entity
                && agent.output_type == AgentOutputType::Json
                && !outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
        }) {
            agent.append_user_message(&prompt);
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn protagonist_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut ProtagonistDecisionState)>,
    mut agents: Query<
        (Entity, &mut Agent, &SessionOwner),
        (With<Applicator>, With<RunningReasoning>),
    >,
    mut agent_tasks: ResMut<AgentTaskManager>,
) {
    for (session_entity, mut flow, mut decision_state) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::Application)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity && agent.output_type == AgentOutputType::Json
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
                    commands.entity(entity).remove::<RunningReasoning>().insert(
                        ApplicationCompleted {
                            turn_id: flow.active_turn_id,
                        },
                    );
                }
                TaskStatus::Error => {
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
