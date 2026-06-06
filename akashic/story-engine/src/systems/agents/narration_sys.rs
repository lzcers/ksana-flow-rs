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
        outcome::NarrationOutcome,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        agent_task::{AgentTaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

#[allow(clippy::type_complexity)]
pub fn narration_dispatch_system(
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
        let prompt = format!(
            "{}\n",
            world_snapshot.to_story_prompt(Some(decision_state.committed_action())),
        );

        for (entity, mut agent, _, _) in agents.iter_mut().filter(|(_, agent, owner, outcome)| {
            owner.0 == session_entity
                && agent.output_type == AgentOutputType::Text
                && !outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id)
        }) {
            agent.append_user_message(&prompt);
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn narration_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    mut agents: Query<
        (Entity, &mut Agent, &SessionOwner),
        (With<Applicator>, With<RunningReasoning>),
    >,
    mut agent_tasks: ResMut<AgentTaskManager>,
) {
    for (session_entity, mut flow) in sessions
        .iter_mut()
        .filter(|(_, flow)| flow.stage == TurnStage::Application)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity && agent.output_type == AgentOutputType::Text
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
                    agent.append_assistant_message(&output);
                    commands
                        .entity(entity)
                        .remove::<RunningReasoning>()
                        .insert(NarrationOutcome {
                            turn_id: flow.active_turn_id,
                            content: output,
                        })
                        .insert(ApplicationCompleted {
                            turn_id: flow.active_turn_id,
                        });
                }
                TaskStatus::Error => {
                    agent_tasks.clear_task(entity);
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
