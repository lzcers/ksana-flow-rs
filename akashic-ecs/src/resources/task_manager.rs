use std::collections::HashMap;

use agent::agent::Context;
use bevy_ecs::{entity::Entity, resource::Resource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    FateWeaving,
    ProtagonistAction,
    Narration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub owner: Entity,
    pub kind: TaskKind,
    pub status: TaskStatus,
}

#[derive(Resource, Default)]
pub struct TaskManager {
    tasks: HashMap<Entity, TaskSnapshot>,
}

impl TaskManager {
    pub fn spawn_task(&mut self, owner: Entity, kind: TaskKind, _context: &Context) {
        self.tasks.insert(
            owner,
            TaskSnapshot {
                owner,
                kind,
                status: TaskStatus::Pending,
            },
        );
    }

    pub fn poll_all_tasks(&mut self) {
        for task in self.tasks.values_mut() {
            task.status = match task.status {
                TaskStatus::Pending => TaskStatus::Running,
                TaskStatus::Running => TaskStatus::Done,
                TaskStatus::Done => TaskStatus::Done,
                TaskStatus::Error => TaskStatus::Error,
            };
        }
    }

    pub fn task_status(&self, owner: Entity) -> Option<TaskStatus> {
        self.tasks.get(&owner).map(|task| task.status)
    }

    pub fn task_snapshot(&self, owner: Entity) -> Option<TaskSnapshot> {
        self.tasks.get(&owner).copied()
    }

    pub fn clear_task(&mut self, owner: Entity) {
        self.tasks.remove(&owner);
    }
}
