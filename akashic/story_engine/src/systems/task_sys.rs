use bevy_ecs::system::{Query, ResMut};

use crate::{
    components::agent::SessionOwner,
    resources::{export::ExportState, llm_task_manager::TaskManager},
};

pub fn task_poll_system(
    mut task_manager: ResMut<TaskManager>,
    agent_owners: Query<&SessionOwner>,
    export_states: Query<&ExportState>,
) {
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
