use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    query::With,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::agent::{Agent, AgentOutputType, Applicator, PendingReasoning, Player, Simulator},
    resources::{
        agent_task::{AgentTaskManager, TaskKind},
        export::ExportState,
    },
};

#[allow(clippy::type_complexity)]
pub fn agent_task_system(
    mut commands: Commands,
    mut agent_tasks: ResMut<AgentTaskManager>,
    pending_simulators: Query<(Entity, &Agent), (With<PendingReasoning>, With<Simulator>)>,
    pending_applicators: Query<(Entity, &Agent), (With<PendingReasoning>, With<Applicator>)>,
    pending_players: Query<Entity, (With<PendingReasoning>, With<Player>)>,
    agent_owners: Query<&ChildOf>,
    export_states: Query<&ExportState>,
) {
    for (entity, agent) in pending_simulators.iter() {
        agent_tasks.spawn_task(entity, TaskKind::Simulation, &agent.context);
        commands.entity(entity).remove::<PendingReasoning>();
    }

    for (entity, agent) in pending_applicators.iter() {
        let task_kind = match agent.output_type {
            AgentOutputType::Text => TaskKind::Narration,
            AgentOutputType::Json => TaskKind::ProtagonistAction,
        };
        agent_tasks.spawn_task(entity, task_kind, &agent.context);
        commands.entity(entity).remove::<PendingReasoning>();
    }

    for entity in pending_players.iter() {
        commands.entity(entity).remove::<PendingReasoning>();
    }

    agent_tasks.poll_all_tasks();
    for (agent_entity, update) in agent_tasks.take_updates() {
        let Ok(owner) = agent_owners.get(agent_entity) else {
            continue;
        };
        let Ok(export_state) = export_states.get(owner.parent()) else {
            continue;
        };
        export_state.publish_task_update(update);
    }
}
