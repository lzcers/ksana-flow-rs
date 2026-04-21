use std::{
    collections::HashMap,
    pin::Pin,
    sync::Mutex,
    task::{Context as PollContext, Poll},
};

use agent::{
    agent::{CallModelEvent, Context, call_model},
    models::ChatModel,
};
use async_stream::stream;
use bevy_ecs::{entity::Entity, resource::Resource};
use futures::{Stream, StreamExt, task::noop_waker_ref};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    FateWeaving,
    ProtagonistAction,
    Narration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Error,
}

#[derive(Resource)]
pub struct TaskManager {
    model: ChatModel,
    tasks: HashMap<Entity, TaskHandle>,
}

type TaskStream = Pin<Box<dyn Stream<Item = CallModelEvent> + Send>>;
type TaskResult = Result<String, String>;

pub struct TaskHandle {
    kind: TaskKind,
    stream: Mutex<TaskStream>,
    chunks: Vec<String>,
    result: Option<TaskResult>,
    status: TaskStatus,
    finished: bool,
}

#[derive(Clone, Debug)]
pub struct TaskSnapshot {
    pub owner: Entity,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub chunks: Vec<String>,
    pub result: Option<TaskResult>,
    pub finished: bool,
}

impl TaskHandle {
    fn new(kind: TaskKind, stream: TaskStream) -> Self {
        Self {
            kind,
            stream: Mutex::new(stream),
            chunks: Vec::new(),
            result: None,
            status: TaskStatus::Pending,
            finished: false,
        }
    }

    fn snapshot(&self, owner: Entity) -> TaskSnapshot {
        TaskSnapshot {
            owner,
            kind: self.kind,
            status: self.status,
            chunks: self.chunks.clone(),
            result: self.result.clone(),
            finished: self.finished,
        }
    }
}

impl TaskManager {
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
        }
    }

    // 创建一个任务
    pub fn spawn_task(&mut self, entity: Entity, kind: TaskKind, ctx: &Context) {
        let msgs = ctx.to_messages();
        let model = self.model.clone();
        let stream = Box::pin(stream! {
            let mut inner_stream = std::pin::pin!(call_model(&model, &msgs, None));
            while let Some(event) = inner_stream.next().await {
                yield event;
            }
        });

        self.tasks.insert(entity, TaskHandle::new(kind, stream));
    }

    pub fn poll_all_tasks(&mut self) {
        let task_entities: Vec<Entity> = self.tasks.keys().copied().collect();
        for entity in task_entities {
            let _ = self.poll_task(entity);
        }
    }

    pub fn poll_task(&mut self, entity: Entity) -> TaskStatus {
        let Some(task) = self.tasks.get_mut(&entity) else {
            return TaskStatus::Error;
        };

        if task.finished {
            task.status = match task.result {
                Some(Ok(_)) => TaskStatus::Done,
                Some(Err(_)) | None => TaskStatus::Error,
            };
            return task.status;
        }

        let waker = noop_waker_ref();
        let mut cx = PollContext::from_waker(waker);
        let mut stream = task.stream.lock().expect("task stream poisoned");

        loop {
            match stream.as_mut().poll_next(&mut cx) {
                Poll::Ready(Some(CallModelEvent::TextChunk(content))) => {
                    task.status = TaskStatus::Running;
                    task.chunks.push(content);
                }
                Poll::Ready(Some(CallModelEvent::Completed { content, .. })) => {
                    task.result = Some(Ok(content));
                    task.status = TaskStatus::Done;
                    task.finished = true;
                    return TaskStatus::Done;
                }
                Poll::Ready(Some(CallModelEvent::Error(error))) => {
                    task.result = Some(Err(error));
                    task.status = TaskStatus::Error;
                    task.finished = true;
                    return TaskStatus::Error;
                }
                Poll::Ready(Some(CallModelEvent::ReasoningChunk(_))) => {}
                Poll::Ready(None) => {
                    task.result
                        .get_or_insert_with(|| Err("task ended without completion".to_string()));
                    task.status = TaskStatus::Error;
                    task.finished = true;
                    return TaskStatus::Error;
                }
                Poll::Pending => {
                    task.status = TaskStatus::Running;
                    return TaskStatus::Running;
                }
            }
        }
    }

    pub fn task_status(&self, entity: Entity) -> Option<TaskStatus> {
        self.tasks.get(&entity).map(|task| task.status)
    }

    pub fn task_snapshot(&self, entity: Entity) -> Option<TaskSnapshot> {
        self.tasks.get(&entity).map(|task| task.snapshot(entity))
    }

    pub fn clear_task(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
    }
}
