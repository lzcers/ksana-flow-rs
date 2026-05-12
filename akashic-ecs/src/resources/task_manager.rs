use std::{
    collections::HashMap,
    pin::{Pin, pin},
    sync::{Arc, Mutex},
    task::{Context as PollContext, Poll},
    time::{Duration, Instant},
};

use agent::{
    agent::{CallModelEvent, Context, call_model},
    core::Message,
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
const TASK_MAX_RETRIES: usize = 2;

pub struct TaskHandle {
    stream: Mutex<TaskStream>,
    restart_stream: Arc<dyn Fn() -> TaskStream + Send + Sync>,
    started_at: Instant,
    last_progress_at: Instant,
    attempts: usize,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub attempts: usize,
    pub max_attempts: usize,
    pub last_error: Option<String>,
    pub retry_history: Vec<String>,
    pub chunks: Vec<String>,
    pub result: Option<Result<String, String>>,
}

impl TaskHandle {
    fn new(stream: TaskStream, restart_stream: Arc<dyn Fn() -> TaskStream + Send + Sync>) -> Self {
        let now = Instant::now();
        Self {
            stream: Mutex::new(stream),
            restart_stream,
            started_at: now,
            last_progress_at: now,
            attempts: 1,
        }
    }

    fn restart(&mut self) -> usize {
        let now = Instant::now();
        self.stream = Mutex::new((self.restart_stream)());
        self.started_at = now;
        self.last_progress_at = now;
        self.attempts += 1;
        self.attempts
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
        let restart_stream = Self::make_task_stream(model, msgs);
        let stream = restart_stream();

        self.results.insert(
            entity,
            TaskResult {
                kind,
                status: TaskStatus::Pending,
                attempts: 1,
                max_attempts: TASK_MAX_RETRIES + 1,
                last_error: None,
                retry_history: Vec::new(),
                chunks: Vec::new(),
                result: None,
            },
        );
        self.tasks
            .insert(entity, TaskHandle::new(stream, restart_stream));
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
        let failure = {
            let Some(result) = self.results.get_mut(&entity) else {
                return TaskStatus::Error;
            };

            if matches!(result.status, TaskStatus::Done | TaskStatus::Error) {
                return result.status;
            }

            let Some(task) = self.tasks.get_mut(&entity) else {
                result.status = TaskStatus::Error;
                result.last_error = Some("task handle missing".to_string());
                result
                    .result
                    .get_or_insert_with(|| Err("task handle missing".to_string()));
                return TaskStatus::Error;
            };

            let now = Instant::now();
            if now.duration_since(task.started_at) > TASK_TIMEOUT {
                Some(format!(
                    "task timed out after {:?} without completing",
                    TASK_TIMEOUT
                ))
            } else {
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
                            result.last_error = None;
                            task.last_progress_at = Instant::now();
                            result.attempts = task.attempts;
                            return TaskStatus::Done;
                        }
                        Poll::Ready(Some(CallModelEvent::Error(error))) => {
                            break Some(error);
                        }
                        Poll::Ready(Some(CallModelEvent::ReasoningChunk(_))) => {
                            // Some providers stream reasoning before user-visible text.
                            // Treat it as progress so long-thinking requests are not retried early.
                            result.status = TaskStatus::Running;
                            task.last_progress_at = Instant::now();
                        }
                        Poll::Ready(None) => {
                            break Some("task ended without completion".to_string());
                        }
                        Poll::Pending => {
                            if Instant::now().duration_since(task.last_progress_at)
                                > TASK_NO_PROGRESS_TIMEOUT
                            {
                                break Some(format!(
                                    "task stalled for {:?} without new output",
                                    TASK_NO_PROGRESS_TIMEOUT
                                ));
                            }
                            result.status = TaskStatus::Running;
                            result.attempts = task.attempts;
                            return TaskStatus::Running;
                        }
                    }
                }
            }
        };

        self.retry_or_fail(entity, failure.expect("task failure should be present"))
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

    fn retry_or_fail(&mut self, entity: Entity, message: String) -> TaskStatus {
        let Some(result) = self.results.get_mut(&entity) else {
            return TaskStatus::Error;
        };

        let Some(task) = self.tasks.get_mut(&entity) else {
            result.status = TaskStatus::Error;
            result.last_error = Some(message.clone());
            result.retry_history.push(message.clone());
            result.result = Some(Err(message));
            return TaskStatus::Error;
        };

        if task.attempts <= TASK_MAX_RETRIES {
            let next_attempt = task.restart();
            result.status = TaskStatus::Pending;
            result.attempts = next_attempt;
            result.last_error = Some(message.clone());
            result.retry_history.push(message);
            result.result = None;
            return TaskStatus::Pending;
        }

        result.status = TaskStatus::Error;
        result.attempts = task.attempts;
        result.last_error = Some(message.clone());
        result.retry_history.push(message.clone());
        result.result = Some(Err(message));
        TaskStatus::Error
    }

    fn make_task_stream(
        model: ChatModel,
        msgs: Vec<Message>,
    ) -> Arc<dyn Fn() -> TaskStream + Send + Sync> {
        Arc::new(move || Self::build_task_stream(model.clone(), msgs.clone()))
    }

    fn build_task_stream(model: ChatModel, msgs: Vec<Message>) -> TaskStream {
        Box::pin(stream! {
            let mut inner_stream = pin!(call_model(&model, &msgs, None));
            while let Some(event) = inner_stream.next().await {
                yield event;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    use futures::stream;

    fn make_manager() -> TaskManager {
        TaskManager {
            model: ChatModel::new(),
            tasks: HashMap::new(),
            results: HashMap::new(),
        }
    }

    fn make_result(kind: TaskKind, status: TaskStatus) -> TaskResult {
        TaskResult {
            kind,
            status,
            attempts: 1,
            max_attempts: TASK_MAX_RETRIES + 1,
            last_error: None,
            retry_history: Vec::new(),
            chunks: Vec::new(),
            result: None,
        }
    }

    fn make_handle_with_retries(
        initial_stream: TaskStream,
        retry_streams: Vec<TaskStream>,
    ) -> TaskHandle {
        let retry_streams = Arc::new(Mutex::new(VecDeque::from(retry_streams)));
        let restart_stream: Arc<dyn Fn() -> TaskStream + Send + Sync> = {
            let retry_streams = Arc::clone(&retry_streams);
            Arc::new(move || {
                retry_streams
                    .lock()
                    .expect("retry stream queue poisoned")
                    .pop_front()
                    .unwrap_or_else(|| Box::pin(stream::pending::<CallModelEvent>()))
            })
        };
        TaskHandle::new(initial_stream, restart_stream)
    }

    #[test]
    fn poll_task_returns_running_for_fresh_pending_stream() {
        let entity = Entity::from_raw_u32(1).expect("valid entity id");
        let mut manager = make_manager();
        manager.tasks.insert(
            entity,
            make_handle_with_retries(Box::pin(stream::pending::<CallModelEvent>()), Vec::new()),
        );
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Pending),
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Running);
        assert_eq!(manager.results[&entity].status, TaskStatus::Running);
    }

    #[test]
    fn poll_task_retries_stalled_stream_and_completes_on_next_attempt() {
        let entity = Entity::from_raw_u32(2).expect("valid entity id");
        let mut manager = make_manager();
        let retry_stream = Box::pin(stream::iter(vec![CallModelEvent::Completed {
            content: "retry succeeded".to_string(),
            reasoning_content: None,
            tools_call: None,
            usage: None,
        }]));
        let mut handle = make_handle_with_retries(
            Box::pin(stream::pending::<CallModelEvent>()),
            vec![retry_stream],
        );
        handle.last_progress_at =
            Instant::now() - TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Pending);
        assert_eq!(manager.results[&entity].status, TaskStatus::Pending);
        assert_eq!(manager.results[&entity].attempts, 2);
        assert!(
            manager.results[&entity]
                .last_error
                .as_ref()
                .is_some_and(|message| message.contains("task stalled"))
        );
        assert_eq!(manager.results[&entity].retry_history.len(), 1);
        assert!(manager.results[&entity].chunks.is_empty());

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].attempts, 2);
        assert_eq!(manager.results[&entity].last_error, None);
        assert_eq!(manager.results[&entity].retry_history.len(), 1);
        assert_eq!(
            manager.results[&entity].result,
            Some(Ok("retry succeeded".to_string()))
        );
    }

    #[test]
    fn poll_task_marks_error_after_retries_exhausted() {
        let entity = Entity::from_raw_u32(3).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle =
            make_handle_with_retries(Box::pin(stream::pending::<CallModelEvent>()), Vec::new());
        handle.started_at = Instant::now() - TASK_TIMEOUT - Duration::from_secs(1);
        handle.attempts = TASK_MAX_RETRIES + 1;
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );
        manager.results.get_mut(&entity).expect("result").attempts = TASK_MAX_RETRIES + 1;

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Error);
        assert_eq!(manager.results[&entity].status, TaskStatus::Error);
        assert_eq!(manager.results[&entity].attempts, TASK_MAX_RETRIES + 1);
        assert_eq!(manager.results[&entity].retry_history.len(), 1);
        assert!(
            manager.results[&entity]
                .last_error
                .as_ref()
                .is_some_and(|message| message.contains("timed out"))
        );
        assert!(
            manager.results[&entity]
                .result
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .is_some_and(|message| message.contains("timed out"))
        );
    }

    #[test]
    fn poll_task_treats_reasoning_chunks_as_progress() {
        let entity = Entity::from_raw_u32(4).expect("valid entity id");
        let mut manager = make_manager();
        let stream = Box::pin(stream::iter(vec![
            CallModelEvent::ReasoningChunk("thinking".to_string()),
            CallModelEvent::Completed {
                content: "done".to_string(),
                reasoning_content: Some("thinking".to_string()),
                tools_call: None,
                usage: None,
            },
        ]));
        let mut handle = make_handle_with_retries(stream, Vec::new());
        handle.last_progress_at =
            Instant::now() - TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].attempts, 1);
        assert_eq!(manager.results[&entity].last_error, None);
        assert!(manager.results[&entity].retry_history.is_empty());
        assert_eq!(
            manager.results[&entity].result,
            Some(Ok("done".to_string()))
        );
    }
}
