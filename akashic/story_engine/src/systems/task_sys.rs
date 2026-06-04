use bevy_ecs::system::{Res, ResMut};

use crate::resources::{export::ExportState, llm_task_manager::TaskManager};

pub fn task_poll_system(mut task_manager: ResMut<TaskManager>, export_state: Res<ExportState>) {
    task_manager.poll_all_tasks();
    for update in task_manager.take_updates() {
        export_state.publish_task_update(update);
    }
}
