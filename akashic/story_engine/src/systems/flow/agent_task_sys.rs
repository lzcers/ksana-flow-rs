use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::agent::{
        Agent, AgentKind, AgentOutputType, PendingReasoning, RunningReasoning, SessionOwner,
    },
    resources::{
        export::ExportState,
        llm_task_manager::{TaskKind, TaskManager},
    },
};

pub fn agent_task_system(
    mut commands: Commands,
    mut task_manager: ResMut<TaskManager>,
    pending_agents: Query<(Entity, &Agent), With<PendingReasoning>>,
    agent_owners: Query<&SessionOwner>,
    export_states: Query<&ExportState>,
) {
    for (entity, agent) in pending_agents.iter() {
        if let Some(task_kind) = task_kind_for(agent.kind, agent.output_type) {
            task_manager.spawn_task(entity, task_kind, &agent.context);
            commands
                .entity(entity)
                .remove::<PendingReasoning>()
                .insert(RunningReasoning);
        } else {
            commands.entity(entity).remove::<PendingReasoning>();
        }
    }

    task_manager.poll_all_tasks();
    for (agent_entity, update) in task_manager.take_updates() {
        let Ok(owner) = agent_owners.get(agent_entity) else {
            continue;
        };
        let Ok(export_state) = export_states.get(owner.0) else {
            continue;
        };
        export_state.publish_task_update(update);
    }
}

fn task_kind_for(kind: AgentKind, output_type: AgentOutputType) -> Option<TaskKind> {
    match (kind, output_type) {
        (AgentKind::Simulator, _) => Some(TaskKind::Simulation),
        (AgentKind::Applicator, AgentOutputType::Text) => Some(TaskKind::Narration),
        (AgentKind::Applicator, AgentOutputType::Json) => Some(TaskKind::ProtagonistAction),
        (AgentKind::Player, _) => None,
    }
}
