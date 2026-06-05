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
pub enum TaskKind {
    Simulation,
    ProtagonistAction,
    Narration,
}

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
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub attempts: usize,
    pub max_attempts: usize,
    pub last_error: Option<String>,
    pub chunks: Vec<String>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdate {
    pub entity: String,
    pub kind: TaskKind,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Resource)]
pub struct TaskManager {
    model: ChatModel,
    tasks: HashMap<Entity, RunningTask>,
    pub results: HashMap<Entity, TaskResult>,
    emitted_updates: Vec<(Entity, TaskUpdate)>,
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
            emitted_updates: Vec::new(),
        }
    }

    pub fn spawn_task(&mut self, entity: Entity, kind: TaskKind, ctx: &Context) {
        if let Some(existing_task) = self.tasks.remove(&entity) {
            existing_task.runtime.handle.abort();
        }

        self.results.insert(entity, TaskResult::pending(kind));
        self.emitted_updates.push((
            entity,
            TaskUpdate {
                entity: task_entity_label(entity),
                kind,
                status: TaskStatus::Pending,
                chunk: None,
                output: None,
                error: None,
            },
        ));
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
            let error = "task handle missing".to_string();
            result.mark_failed(error.clone());
            self.emitted_updates.push((
                entity,
                TaskUpdate {
                    entity: task_entity_label(entity),
                    kind: result.kind,
                    status: TaskStatus::Error,
                    chunk: None,
                    output: None,
                    error: Some(error),
                },
            ));
            return TaskStatus::Error;
        };

        if result.status != TaskStatus::Running {
            result.mark_running();
            self.emitted_updates.push((
                entity,
                TaskUpdate {
                    entity: task_entity_label(entity),
                    kind: result.kind,
                    status: TaskStatus::Running,
                    chunk: None,
                    output: None,
                    error: None,
                },
            ));
        }
        match task.runtime.rx.try_recv() {
            Ok(TaskRuntimeEvent::Completed(content)) => {
                result.mark_done(content.clone());
                self.emitted_updates.push((
                    entity,
                    TaskUpdate {
                        entity: task_entity_label(entity),
                        kind: result.kind,
                        status: TaskStatus::Done,
                        chunk: None,
                        output: Some(content),
                        error: None,
                    },
                ));
                self.tasks.remove(&entity);
                TaskStatus::Done
            }
            Ok(TaskRuntimeEvent::Failed(error)) => {
                result.mark_failed(error.clone());
                self.emitted_updates.push((
                    entity,
                    TaskUpdate {
                        entity: task_entity_label(entity),
                        kind: result.kind,
                        status: TaskStatus::Error,
                        chunk: None,
                        output: None,
                        error: Some(error),
                    },
                ));
                self.tasks.remove(&entity);
                TaskStatus::Error
            }
            Err(TryRecvError::Empty) => TaskStatus::Running,
            Err(TryRecvError::Disconnected) => {
                let error = "task runtime ended without completion".to_string();
                result.mark_failed(error.clone());
                self.emitted_updates.push((
                    entity,
                    TaskUpdate {
                        entity: task_entity_label(entity),
                        kind: result.kind,
                        status: TaskStatus::Error,
                        chunk: None,
                        output: None,
                        error: Some(error),
                    },
                ));
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

    pub fn take_updates(&mut self) -> Vec<(Entity, TaskUpdate)> {
        std::mem::take(&mut self.emitted_updates)
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
    fn pending(kind: TaskKind) -> Self {
        Self {
            kind,
            status: TaskStatus::Pending,
            attempts: 1,
            max_attempts: 1,
            last_error: None,
            chunks: Vec::new(),
            output: None,
            error: None,
        }
    }

    fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
    }

    fn mark_done(&mut self, content: String) {
        self.status = TaskStatus::Done;
        self.last_error = None;
        self.output = Some(content);
        self.error = None;
    }

    fn mark_failed(&mut self, message: String) {
        self.status = TaskStatus::Error;
        self.last_error = Some(message.clone());
        self.output = None;
        self.error = Some(message);
    }
}

fn task_entity_label(entity: Entity) -> String {
    format!("{entity:?}")
}
