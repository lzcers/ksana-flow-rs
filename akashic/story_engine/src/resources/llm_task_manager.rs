use std::collections::HashMap;

use agent::{
    agent::{CallModelEvent, Context, call_model},
    core::Message,
    models::ChatModel,
};
use bevy_ecs::{entity::Entity, resource::Resource};
use futures::StreamExt;
use serde::Serialize;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, error::TryRecvError},
    task::JoinHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Error,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub status: TaskStatus,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Resource)]
pub struct TaskManager {
    model: ChatModel,
    tasks: HashMap<Entity, RunningTask>,
    pub results: HashMap<Entity, TaskResult>,
}

struct RunningTask {
    runtime: TaskRuntime,
}

struct TaskRuntime {
    rx: UnboundedReceiver<TaskRuntimeEvent>,
    handle: JoinHandle<()>,
}

#[derive(Clone, Debug)]
enum TaskRuntimeEvent {
    Completed(String),
    Failed(String),
}

impl TaskManager {
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub fn spawn_task(&mut self, entity: Entity, ctx: &Context) {
        if let Some(existing_task) = self.tasks.remove(&entity) {
            existing_task.runtime.handle.abort();
        }

        self.results.insert(entity, TaskResult::pending());
        self.tasks.insert(
            entity,
            RunningTask {
                runtime: Self::spawn_runtime_task(self.model.clone(), ctx.to_messages()),
            },
        );
    }

    pub fn poll_all_tasks(&mut self) {
        let task_entities: Vec<Entity> = self.tasks.keys().copied().collect();
        for entity in task_entities {
            let _ = self.poll_task(entity);
        }
    }

    pub fn poll_task(&mut self, entity: Entity) -> TaskStatus {
        let Some(result) = self.results.get_mut(&entity) else {
            return TaskStatus::Error;
        };

        if matches!(result.status, TaskStatus::Done | TaskStatus::Error) {
            return result.status;
        }

        let Some(task) = self.tasks.get_mut(&entity) else {
            result.mark_failed("task handle missing".to_string());
            return TaskStatus::Error;
        };

        result.mark_running();
        match task.runtime.rx.try_recv() {
            Ok(TaskRuntimeEvent::Completed(content)) => {
                result.mark_done(content);
                self.tasks.remove(&entity);
                TaskStatus::Done
            }
            Ok(TaskRuntimeEvent::Failed(error)) => {
                result.mark_failed(error);
                self.tasks.remove(&entity);
                TaskStatus::Error
            }
            Err(TryRecvError::Empty) => TaskStatus::Running,
            Err(TryRecvError::Disconnected) => {
                result.mark_failed("task runtime ended without completion".to_string());
                self.tasks.remove(&entity);
                TaskStatus::Error
            }
        }
    }

    pub fn task_result(&self, entity: Entity) -> Option<&TaskResult> {
        self.results.get(&entity)
    }

    pub fn take_result(&mut self, entity: Entity) -> Option<TaskResult> {
        self.results.remove(&entity)
    }

    pub fn clear_task(&mut self, entity: Entity) {
        if let Some(task) = self.tasks.remove(&entity) {
            task.runtime.handle.abort();
        }
        self.results.remove(&entity);
    }

    fn spawn_runtime_task(model: ChatModel, msgs: Vec<Message>) -> TaskRuntime {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(async move {
            let mut stream = Box::pin(call_model(&model, &msgs, None));
            while let Some(event) = stream.next().await {
                match event {
                    CallModelEvent::TextChunk(_) | CallModelEvent::ReasoningChunk(_) => {}
                    CallModelEvent::Completed { content, .. } => {
                        let _ = tx.send(TaskRuntimeEvent::Completed(content));
                        return;
                    }
                    CallModelEvent::Error(error) => {
                        let _ = tx.send(TaskRuntimeEvent::Failed(error));
                        return;
                    }
                }
            }

            let _ = tx.send(TaskRuntimeEvent::Failed(
                "task ended without completion".to_string(),
            ));
        });

        TaskRuntime { rx, handle }
    }
}

impl TaskResult {
    fn pending() -> Self {
        Self {
            status: TaskStatus::Pending,
            output: None,
            error: None,
        }
    }

    fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
    }

    fn mark_done(&mut self, content: String) {
        self.status = TaskStatus::Done;
        self.output = Some(content);
        self.error = None;
    }

    fn mark_failed(&mut self, message: String) {
        self.status = TaskStatus::Error;
        self.output = None;
        self.error = Some(message);
    }
}
