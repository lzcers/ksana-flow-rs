use std::{
    collections::HashMap,
    pin::{Pin, pin},
    sync::Mutex,
    task::{Context as PollContext, Poll},
    time::{Duration, Instant},
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
const TASK_TIMEOUT: Duration = Duration::from_secs(180);
const TASK_NO_PROGRESS_TIMEOUT: Duration = Duration::from_secs(45);

pub struct TaskHandle {
    stream: Mutex<TaskStream>,
    started_at: Instant,
    last_progress_at: Instant,
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
        let now = Instant::now();
        Self {
            stream: Mutex::new(stream),
            started_at: now,
            last_progress_at: now,
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
        model.set_output_json(matches!(
            kind,
            TaskKind::FatePlanning | TaskKind::ProtagonistAction
        ));
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

        let now = Instant::now();
        if now.duration_since(task.started_at) > TASK_TIMEOUT {
            result.status = TaskStatus::Error;
            result.result.get_or_insert_with(|| {
                Err(format!(
                    "task timed out after {:?} without completing",
                    TASK_TIMEOUT
                ))
            });
            return TaskStatus::Error;
        }

        let waker = noop_waker_ref();
        let mut cx = PollContext::from_waker(waker);
        let mut stream = task.stream.lock().expect("task stream poisoned");

        loop {
            match stream.as_mut().poll_next(&mut cx) {
                Poll::Ready(Some(CallModelEvent::TextChunk(content))) => {
                    result.status = TaskStatus::Running;
                    task.last_progress_at = Instant::now();
                    result.chunks.push(content);
                }
                Poll::Ready(Some(CallModelEvent::Completed {
                    content, usage: _, ..
                })) => {
                    result.result = Some(Ok(content));
                    result.status = TaskStatus::Done;
                    task.last_progress_at = Instant::now();
                    return TaskStatus::Done;
                }
                Poll::Ready(Some(CallModelEvent::Error(error))) => {
                    result.result = Some(Err(error));
                    result.status = TaskStatus::Error;
                    task.last_progress_at = Instant::now();
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
                    if Instant::now().duration_since(task.last_progress_at) > TASK_NO_PROGRESS_TIMEOUT
                    {
                        result.status = TaskStatus::Error;
                        result.result.get_or_insert_with(|| {
                            Err(format!(
                                "task stalled for {:?} without new output",
                                TASK_NO_PROGRESS_TIMEOUT
                            ))
                        });
                        return TaskStatus::Error;
                    }
                    result.status = TaskStatus::Running;
                    return TaskStatus::Running;
                }
            }
        }
    }

    pub fn task_result(&self, entity: Entity) -> Option<TaskResult> {
        self.results.get(&entity).cloned()
    }

    pub fn clear_task(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
        self.results.remove(&entity);
    }

    pub fn task_results_snapshot(&self) -> Vec<(Entity, TaskResult)> {
        self.results
            .iter()
            .map(|(entity, result)| (*entity, result.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    fn make_manager() -> TaskManager {
        TaskManager {
            model: ChatModel::new(),
            tasks: HashMap::new(),
            results: HashMap::new(),
        }
    }

    #[test]
    fn poll_task_returns_running_for_fresh_pending_stream() {
        let entity = Entity::from_raw_u32(1).expect("valid entity id");
        let mut manager = make_manager();
        manager.tasks.insert(
            entity,
            TaskHandle::new(Box::pin(stream::pending::<CallModelEvent>())),
        );
        manager.results.insert(
            entity,
            TaskResult {
                kind: TaskKind::Narration,
                status: TaskStatus::Pending,
                chunks: Vec::new(),
                result: None,
            },
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Running);
        assert_eq!(manager.results[&entity].status, TaskStatus::Running);
    }

    #[test]
    fn poll_task_marks_error_when_stream_stalls() {
        let entity = Entity::from_raw_u32(2).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle = TaskHandle::new(Box::pin(stream::pending::<CallModelEvent>()));
        handle.last_progress_at = Instant::now() - TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            TaskResult {
                kind: TaskKind::Narration,
                status: TaskStatus::Running,
                chunks: Vec::new(),
                result: None,
            },
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Error);
        assert_eq!(manager.results[&entity].status, TaskStatus::Error);
        assert!(
            manager.results[&entity]
                .result
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .is_some_and(|message| message.contains("task stalled"))
        );
    }
}
