use agent::agent::Context;
use bevy_ecs::{entity::Entity, resource::Resource};

#[derive(Resource)]
pub struct TaskManager {}

pub enum TaskStatus {
    Running,
    Done,
    Error,
}

impl TaskManager {
    // 创建一个任务
    pub fn spawn_task(&mut self, entity: Entity, context: &Context) {}
    pub fn poll_task(&mut self, entity: Entity) -> TaskStatus {
        todo!()
    }
}
