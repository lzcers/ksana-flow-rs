use std::{
    collections::HashMap,
    pin::{Pin, pin},
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
    FatePlanning,
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
    results: HashMap<Entity, TaskResult>,
}

type TaskStream = Pin<Box<dyn Stream<Item = CallModelEvent> + Send>>;

pub struct TaskHandle {
    stream: Mutex<TaskStream>,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub chunks: Vec<String>,
    pub result: Option<Result<String, String>>,
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
            results: HashMap::new(),
        }
    }

    // 创建一个任务
    pub fn spawn_task(&mut self, entity: Entity, kind: TaskKind, ctx: &Context) {
        let msgs = ctx.to_messages();
        let model = self.model_for_kind(kind);
        // println!("spawn task {:?}", kind);
        // println!("{}", serde_json::to_string(&msgs).unwrap());
        let stream = Box::pin(stream! {
            let mut inner_stream = pin!(call_model(&model, &msgs, None));
            while let Some(event) = inner_stream.next().await {
                yield event;
            }
        });

        self.results.insert(
            entity,
            TaskResult {
                kind,
                status: TaskStatus::Pending,
                chunks: Vec::new(),
                result: None,
            },
        );
        self.tasks.insert(entity, TaskHandle::new(stream));
    }

    fn model_for_kind(&self, kind: TaskKind) -> ChatModel {
        let mut model = self.model.clone();
        model.set_output_json(matches!(kind, TaskKind::FatePlanning));
        model
    }

    pub fn poll_all_tasks(&mut self) {
        let task_entities: Vec<Entity> = self.tasks.keys().copied().collect();
        for entity in task_entities {
            let _ = self.poll_task(entity);
        }
    }

    pub fn poll_task(&mut self, entity: Entity) -> TaskStatus {
        let (tasks, results) = (&mut self.tasks, &mut self.results);
        let Some(result) = results.get_mut(&entity) else {
            return TaskStatus::Error;
        };

        if matches!(result.status, TaskStatus::Done | TaskStatus::Error) {
            return result.status;
        }

        let Some(task) = tasks.get_mut(&entity) else {
            result.status = TaskStatus::Error;
            result
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
                    result.status = TaskStatus::Running;
                    result.chunks.push(content);
                }
                Poll::Ready(Some(CallModelEvent::Completed { content, usage, .. })) => {
                    result.result = Some(Ok(content));
                    result.status = TaskStatus::Done;
                    return TaskStatus::Done;
                }
                Poll::Ready(Some(CallModelEvent::Error(error))) => {
                    result.result = Some(Err(error));
                    result.status = TaskStatus::Error;
                    return TaskStatus::Error;
                }
                Poll::Ready(Some(CallModelEvent::ReasoningChunk(_))) => {}
                Poll::Ready(None) => {
                    result
                        .result
                        .get_or_insert_with(|| Err("task ended without completion".to_string()));
                    result.status = TaskStatus::Error;
                    return TaskStatus::Error;
                }
                Poll::Pending => {
                    result.status = TaskStatus::Running;
                    return TaskStatus::Running;
                }
            }
        }
    }

    pub fn task_result(&self, entity: Entity) -> Option<TaskResult> {
        self.results.get(&entity).cloned()
    }

    pub fn task_results_snapshot(&self) -> Vec<(Entity, TaskResult)> {
        self.results
            .iter()
            .map(|(entity, result)| (*entity, result.clone()))
            .collect()
    }

    pub fn clear_task(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
        self.results.remove(&entity);
    }
}
