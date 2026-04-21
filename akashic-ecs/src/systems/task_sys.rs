use bevy_ecs::system::ResMut;

use crate::resources::task_manager::TaskManager;

// 通用轮询层：只推进任务流，不解释业务含义。
pub fn task_system(mut task_manager: ResMut<TaskManager>) {
    task_manager.poll_all_tasks();
}
