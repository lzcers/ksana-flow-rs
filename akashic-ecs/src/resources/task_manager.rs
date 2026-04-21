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
    snapshots: HashMap<Entity, TaskSnapshot>,
}

type TaskStream = Pin<Box<dyn Stream<Item = CallModelEvent> + Send>>;
type TaskResult = Result<String, String>;

pub struct TaskHandle {
    stream: Mutex<TaskStream>,
}

#[derive(Clone, Debug)]
pub struct TaskSnapshot {
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub chunks: Vec<String>,
    pub result: Option<TaskResult>,
}

impl TaskHandle {
    fn new(stream: TaskStream) -> Self {
        Self {
            stream: Mutex::new(stream),
        }
    }
}

impl TaskManager {
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
            snapshots: HashMap::new(),
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

        self.snapshots.insert(
            entity,
            TaskSnapshot {
                kind,
                status: TaskStatus::Pending,
                chunks: Vec::new(),
                result: None,
            },
        );
        self.tasks.insert(entity, TaskHandle::new(stream));
    }

    pub fn poll_all_tasks(&mut self) {
        let task_entities: Vec<Entity> = self.tasks.keys().copied().collect();
        for entity in task_entities {
            let _ = self.poll_task(entity);
        }
    }

    pub fn poll_task(&mut self, entity: Entity) -> TaskStatus {
        let (tasks, snapshots) = (&mut self.tasks, &mut self.snapshots);
        let Some(snapshot) = snapshots.get_mut(&entity) else {
            return TaskStatus::Error;
        };

        if matches!(snapshot.status, TaskStatus::Done | TaskStatus::Error) {
            return snapshot.status;
        }

        let Some(task) = tasks.get_mut(&entity) else {
            snapshot.status = TaskStatus::Error;
            snapshot
                .result
                .get_or_insert_with(|| Err("task handle missing".to_string()));
            return TaskStatus::Error;
        };

        let waker = noop_waker_ref();
        let mut cx = PollContext::from_waker(waker);
        let mut stream = task.stream.lock().expect("task stream poisoned");

        loop {
            match stream.as_mut().poll_next(&mut cx) {
                Poll::Ready(Some(CallModelEvent::TextChunk(content))) => {
                    snapshot.status = TaskStatus::Running;
                    snapshot.chunks.push(content);
                }
                Poll::Ready(Some(CallModelEvent::Completed { content, .. })) => {
                    snapshot.result = Some(Ok(content));
                    snapshot.status = TaskStatus::Done;
                    return TaskStatus::Done;
                }
                Poll::Ready(Some(CallModelEvent::Error(error))) => {
                    snapshot.result = Some(Err(error));
                    snapshot.status = TaskStatus::Error;
                    return TaskStatus::Error;
                }
                Poll::Ready(Some(CallModelEvent::ReasoningChunk(_))) => {}
                Poll::Ready(None) => {
                    snapshot
                        .result
                        .get_or_insert_with(|| Err("task ended without completion".to_string()));
                    snapshot.status = TaskStatus::Error;
                    return TaskStatus::Error;
                }
                Poll::Pending => {
                    snapshot.status = TaskStatus::Running;
                    return TaskStatus::Running;
                }
            }
        }
    }

    pub fn task_status(&self, entity: Entity) -> Option<TaskStatus> {
        self.snapshots.get(&entity).map(|task| task.status)
    }

    pub fn task_snapshot(&self, entity: Entity) -> Option<TaskSnapshot> {
        self.snapshots.get(&entity).cloned()
    }

    pub fn clear_task(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
        self.snapshots.remove(&entity);
    }
}
