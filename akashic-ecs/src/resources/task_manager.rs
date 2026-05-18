use std::{
    collections::HashMap,
    env,
    sync::Arc,
    time::{Duration, Instant},
};

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
    FatePlanning,
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

#[derive(Resource)]
pub struct TaskManager {
    model: ChatModel,
    // 运行态只保存仍需被 ECS 收割的后台任务句柄。
    tasks: HashMap<Entity, RunningTask>,
    // 结果快照在终态后仍保留，供 apply/export 等系统读取。
    pub results: HashMap<Entity, TaskResult>,
    pub emitted_updates: Vec<TaskUpdate>,
    config: TaskManagerConfig,
}

const DEFAULT_TASK_TIMEOUT: Duration = Duration::from_secs(600);
const DEFAULT_TASK_INITIAL_OUTPUT_TIMEOUT: Duration = Duration::from_secs(300);
const DEFAULT_TASK_NO_PROGRESS_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TASK_MAX_RETRIES: usize = 2;

#[derive(Clone, Copy, Debug)]
struct TaskManagerConfig {
    task_timeout: Duration,
    initial_output_timeout: Duration,
    no_progress_timeout: Duration,
    max_retries: usize,
}

struct TaskRuntime {
    rx: UnboundedReceiver<TaskRuntimeEvent>,
    handle: JoinHandle<()>,
}

type RuntimeFactory = Arc<dyn Fn() -> TaskRuntime + Send + Sync>;

pub struct RunningTask {
    // runtime 与可观测结果分离：后台任务只产出事件，主线程统一落地到 TaskResult。
    runtime: TaskRuntime,
    restart_runtime: RuntimeFactory,
    started_at: Instant,
    last_progress_at: Instant,
    saw_progress: bool,
    attempts: usize,
}

#[derive(Clone, Debug)]
enum TaskRuntimeEvent {
    TextChunk(String),
    ReasoningChunk,
    Completed(String),
    Failed(String),
}

enum TaskPollOutcome {
    Running,
    Done,
    Failed(String),
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

fn task_entity_label(entity: Entity) -> String {
    format!("{entity:?}")
}

impl TaskKind {
    pub fn stage_name(self) -> &'static str {
        match self {
            TaskKind::FatePlanning => "fate_weaver",
            TaskKind::ProtagonistAction => "protagonist",
            TaskKind::Narration => "upper_narrator",
        }
    }
}

impl RunningTask {
    fn new(runtime: TaskRuntime, restart_runtime: RuntimeFactory) -> Self {
        let now = Instant::now();
        Self {
            runtime,
            restart_runtime,
            started_at: now,
            last_progress_at: now,
            saw_progress: false,
            attempts: 1,
        }
    }

    fn restart(&mut self) -> usize {
        self.runtime.handle.abort();
        let now = Instant::now();
        self.runtime = (self.restart_runtime)();
        self.started_at = now;
        self.last_progress_at = now;
        self.saw_progress = false;
        self.attempts += 1;
        self.attempts
    }

    fn abort(&self) {
        self.runtime.handle.abort();
    }

    fn mark_progress(&mut self) {
        self.last_progress_at = Instant::now();
        self.saw_progress = true;
    }

    fn has_timed_out(&self, timeout: Duration) -> bool {
        Instant::now().duration_since(self.started_at) > timeout
    }

    fn stall_timeout(&self, config: TaskManagerConfig) -> Duration {
        if self.saw_progress {
            config.no_progress_timeout
        } else {
            config.initial_output_timeout
        }
    }

    fn stall_message(&self, config: TaskManagerConfig) -> Option<String> {
        let stall_timeout = self.stall_timeout(config);
        if Instant::now().duration_since(self.last_progress_at) <= stall_timeout {
            return None;
        }

        Some(if self.saw_progress {
            format!("task stalled for {:?} without new output", stall_timeout)
        } else {
            format!(
                "task produced no output for {:?} after start",
                stall_timeout
            )
        })
    }
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            task_timeout: read_env_duration("AKASHIC_TASK_TIMEOUT_SECS", DEFAULT_TASK_TIMEOUT),
            initial_output_timeout: read_env_duration(
                "AKASHIC_TASK_INITIAL_OUTPUT_TIMEOUT_SECS",
                DEFAULT_TASK_INITIAL_OUTPUT_TIMEOUT,
            ),
            no_progress_timeout: read_env_duration(
                "AKASHIC_TASK_NO_PROGRESS_TIMEOUT_SECS",
                DEFAULT_TASK_NO_PROGRESS_TIMEOUT,
            ),
            max_retries: read_env_usize("AKASHIC_TASK_MAX_RETRIES", DEFAULT_TASK_MAX_RETRIES),
        }
    }
}

fn read_env_duration(name: &str, default: Duration) -> Duration {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(default)
}

fn read_env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

impl TaskManager {
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
            results: HashMap::new(),
            emitted_updates: Vec::new(),
            config: TaskManagerConfig::default(),
        }
    }

    // 创建一个任务
    pub fn spawn_task(&mut self, entity: Entity, kind: TaskKind, ctx: &Context) {
        let msgs = ctx.to_messages();
        let model = self.model_for_kind(kind);
        let restart_runtime = Self::make_runtime_factory(model, msgs);
        let runtime = restart_runtime();

        if let Some(existing_task) = self.tasks.remove(&entity) {
            existing_task.abort();
        }

        self.results
            .insert(entity, TaskResult::new(kind, self.config.max_retries));
        self.emitted_updates.push(TaskUpdate {
            entity: task_entity_label(entity),
            kind,
            status: TaskStatus::Pending,
            chunk: None,
            output: None,
            error: None,
        });
        self.tasks
            .insert(entity, RunningTask::new(runtime, restart_runtime));
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
        let config = self.config;
        let emitted_updates = &mut self.emitted_updates;
        let results = &mut self.results;
        let tasks = &mut self.tasks;

        let failure = {
            let Some(result) = results.get_mut(&entity) else {
                return TaskStatus::Error;
            };

            if matches!(result.status, TaskStatus::Done | TaskStatus::Error) {
                return result.status;
            }

            let Some(task) = tasks.get_mut(&entity) else {
                return result.mark_missing_handle();
            };

            // 总超时先于细粒度事件收割，避免永远等待一个失活的 runtime。
            if task.has_timed_out(config.task_timeout) {
                Some(format!(
                    "task timed out after {:?} without completing",
                    config.task_timeout
                ))
            } else {
                match drain_runtime_events(entity, task, result, emitted_updates, config) {
                    TaskPollOutcome::Running => return TaskStatus::Running,
                    TaskPollOutcome::Done => {
                        self.tasks.remove(&entity);
                        return TaskStatus::Done;
                    }
                    TaskPollOutcome::Failed(message) => Some(message),
                }
            }
        };

        let status = self.retry_or_fail(entity, failure.expect("task failure should be present"));
        if matches!(status, TaskStatus::Done | TaskStatus::Error) {
            // 终态后只保留结果快照，运行态句柄立即回收。
            self.tasks.remove(&entity);
        }
        status
    }

    pub fn clear_task(&mut self, entity: Entity) {
        if let Some(task) = self.tasks.remove(&entity) {
            task.abort();
        }
        self.results.remove(&entity);
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

        if task.attempts <= self.config.max_retries {
            let next_attempt = task.restart();
            result.mark_retry_pending(message, next_attempt);
            push_task_update(
                &mut self.emitted_updates,
                entity,
                result.kind,
                TaskStatus::Pending,
                None,
                None,
                result.last_error.clone(),
            );
            return TaskStatus::Pending;
        }

        result.mark_failed(message, task.attempts);
        push_task_update(
            &mut self.emitted_updates,
            entity,
            result.kind,
            TaskStatus::Error,
            None,
            None,
            result.last_error.clone(),
        );
        TaskStatus::Error
    }

    fn make_runtime_factory(model: ChatModel, msgs: Vec<Message>) -> RuntimeFactory {
        Arc::new(move || Self::spawn_runtime_task(model.clone(), msgs.clone()))
    }

    fn spawn_runtime_task(model: ChatModel, msgs: Vec<Message>) -> TaskRuntime {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(async move {
            let mut stream = Box::pin(call_model(&model, &msgs, None));
            while let Some(event) = stream.next().await {
                let runtime_event = match event {
                    CallModelEvent::TextChunk(content) => TaskRuntimeEvent::TextChunk(content),
                    CallModelEvent::ReasoningChunk(_) => TaskRuntimeEvent::ReasoningChunk,
                    CallModelEvent::Completed { content, .. } => {
                        let _ = tx.send(TaskRuntimeEvent::Completed(content));
                        return;
                    }
                    CallModelEvent::Error(error) => TaskRuntimeEvent::Failed(error),
                };
                if tx.send(runtime_event).is_err() {
                    return;
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
    fn new(kind: TaskKind, max_retries: usize) -> Self {
        Self {
            kind,
            status: TaskStatus::Pending,
            attempts: 1,
            max_attempts: max_retries + 1,
            last_error: None,
            retry_history: Vec::new(),
            chunks: Vec::new(),
            result: None,
        }
    }

    fn mark_running(&mut self, attempts: usize) {
        self.status = TaskStatus::Running;
        self.attempts = attempts;
    }

    fn mark_done(&mut self, content: String, attempts: usize) {
        self.result = Some(Ok(content));
        self.status = TaskStatus::Done;
        self.last_error = None;
        self.attempts = attempts;
    }

    fn mark_retry_pending(&mut self, message: String, attempts: usize) {
        self.status = TaskStatus::Pending;
        self.attempts = attempts;
        self.last_error = Some(message.clone());
        self.retry_history.push(message);
        self.result = None;
    }

    fn mark_failed(&mut self, message: String, attempts: usize) {
        self.status = TaskStatus::Error;
        self.attempts = attempts;
        self.last_error = Some(message.clone());
        self.retry_history.push(message.clone());
        self.result = Some(Err(message));
    }

    fn mark_missing_handle(&mut self) -> TaskStatus {
        let message = "task handle missing".to_string();
        self.status = TaskStatus::Error;
        self.last_error = Some(message.clone());
        self.result.get_or_insert(Err(message));
        TaskStatus::Error
    }
}

fn drain_runtime_events(
    entity: Entity,
    task: &mut RunningTask,
    result: &mut TaskResult,
    emitted_updates: &mut Vec<TaskUpdate>,
    config: TaskManagerConfig,
) -> TaskPollOutcome {
    loop {
        match task.runtime.rx.try_recv() {
            Ok(TaskRuntimeEvent::TextChunk(content)) => {
                task.mark_progress();
                result.mark_running(task.attempts);
                result.chunks.push(content.clone());
                push_task_update(
                    emitted_updates,
                    entity,
                    result.kind,
                    TaskStatus::Running,
                    Some(content),
                    None,
                    None,
                );
            }
            Ok(TaskRuntimeEvent::Completed(content)) => {
                task.mark_progress();
                result.mark_done(content.clone(), task.attempts);
                push_task_update(
                    emitted_updates,
                    entity,
                    result.kind,
                    TaskStatus::Done,
                    None,
                    Some(content),
                    None,
                );
                // 已拿到最终结果，后台句柄不再需要继续存活到下一帧。
                task.abort();
                return TaskPollOutcome::Done;
            }
            Ok(TaskRuntimeEvent::Failed(error)) => return TaskPollOutcome::Failed(error),
            Ok(TaskRuntimeEvent::ReasoningChunk) => {
                task.mark_progress();
                result.mark_running(task.attempts);
            }
            Err(TryRecvError::Empty) => {
                if let Some(message) = task.stall_message(config) {
                    return TaskPollOutcome::Failed(message);
                }

                result.mark_running(task.attempts);
                return TaskPollOutcome::Running;
            }
            Err(TryRecvError::Disconnected) => {
                return TaskPollOutcome::Failed(
                    "task runtime ended without completion".to_string(),
                );
            }
        }
    }
}

fn push_task_update(
    emitted_updates: &mut Vec<TaskUpdate>,
    entity: Entity,
    kind: TaskKind,
    status: TaskStatus,
    chunk: Option<String>,
    output: Option<String>,
    error: Option<String>,
) {
    emitted_updates.push(TaskUpdate {
        entity: task_entity_label(entity),
        kind,
        status,
        chunk,
        output,
        error,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, future::pending, sync::Mutex};

    fn make_manager() -> TaskManager {
        TaskManager {
            model: ChatModel::new(),
            tasks: HashMap::new(),
            results: HashMap::new(),
            emitted_updates: Vec::new(),
            config: TaskManagerConfig {
                task_timeout: DEFAULT_TASK_TIMEOUT,
                initial_output_timeout: DEFAULT_TASK_INITIAL_OUTPUT_TIMEOUT,
                no_progress_timeout: DEFAULT_TASK_NO_PROGRESS_TIMEOUT,
                max_retries: DEFAULT_TASK_MAX_RETRIES,
            },
        }
    }

    fn make_result(kind: TaskKind, status: TaskStatus) -> TaskResult {
        TaskResult {
            kind,
            status,
            attempts: 1,
            max_attempts: DEFAULT_TASK_MAX_RETRIES + 1,
            last_error: None,
            retry_history: Vec::new(),
            chunks: Vec::new(),
            result: None,
        }
    }

    enum ScriptedAttempt {
        PendingForever,
        Events(Vec<TaskRuntimeEvent>),
    }

    fn spawn_scripted_runtime(script: ScriptedAttempt) -> TaskRuntime {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(async move {
            match script {
                ScriptedAttempt::PendingForever => {
                    let _tx = tx;
                    pending::<()>().await;
                }
                ScriptedAttempt::Events(events) => {
                    for event in events {
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        TaskRuntime { rx, handle }
    }

    fn make_task_with_retries(
        initial_attempt: ScriptedAttempt,
        retry_attempts: Vec<ScriptedAttempt>,
    ) -> RunningTask {
        let retry_attempts = Arc::new(Mutex::new(VecDeque::from(retry_attempts)));
        let restart_runtime: RuntimeFactory = {
            let retry_attempts = Arc::clone(&retry_attempts);
            Arc::new(move || {
                let next_attempt = retry_attempts
                    .lock()
                    .expect("retry attempt queue poisoned")
                    .pop_front()
                    .unwrap_or(ScriptedAttempt::PendingForever);
                spawn_scripted_runtime(next_attempt)
            })
        };
        RunningTask::new(spawn_scripted_runtime(initial_attempt), restart_runtime)
    }

    #[tokio::test]
    async fn poll_task_returns_running_for_fresh_pending_stream() {
        let entity = Entity::from_raw_u32(1).expect("valid entity id");
        let mut manager = make_manager();
        manager.tasks.insert(
            entity,
            make_task_with_retries(ScriptedAttempt::PendingForever, Vec::new()),
        );
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Pending),
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Running);
        assert_eq!(manager.results[&entity].status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn poll_task_retries_stalled_stream_and_completes_on_next_attempt() {
        let entity = Entity::from_raw_u32(2).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle = make_task_with_retries(
            ScriptedAttempt::PendingForever,
            vec![ScriptedAttempt::Events(vec![TaskRuntimeEvent::Completed(
                "retry succeeded".to_string(),
            )])],
        );
        handle.saw_progress = true;
        handle.last_progress_at =
            Instant::now() - DEFAULT_TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
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

        tokio::task::yield_now().await;
        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].status, TaskStatus::Done);
        assert_eq!(manager.results[&entity].attempts, 2);
        assert_eq!(manager.results[&entity].last_error, None);
        assert_eq!(manager.results[&entity].retry_history.len(), 1);
        assert!(!manager.tasks.contains_key(&entity));
        assert_eq!(
            manager.results[&entity].result,
            Some(Ok("retry succeeded".to_string()))
        );
    }

    #[tokio::test]
    async fn poll_task_marks_error_after_retries_exhausted() {
        let entity = Entity::from_raw_u32(3).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle = make_task_with_retries(ScriptedAttempt::PendingForever, Vec::new());
        handle.started_at = Instant::now() - DEFAULT_TASK_TIMEOUT - Duration::from_secs(1);
        handle.attempts = DEFAULT_TASK_MAX_RETRIES + 1;
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );
        manager.results.get_mut(&entity).expect("result").attempts = DEFAULT_TASK_MAX_RETRIES + 1;

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Error);
        assert_eq!(manager.results[&entity].status, TaskStatus::Error);
        assert_eq!(
            manager.results[&entity].attempts,
            DEFAULT_TASK_MAX_RETRIES + 1
        );
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
        assert!(!manager.tasks.contains_key(&entity));
    }

    #[tokio::test]
    async fn poll_task_treats_reasoning_chunks_as_progress() {
        let entity = Entity::from_raw_u32(4).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle = make_task_with_retries(
            ScriptedAttempt::Events(vec![
                TaskRuntimeEvent::ReasoningChunk,
                TaskRuntimeEvent::Completed("done".to_string()),
            ]),
            Vec::new(),
        );
        handle.last_progress_at =
            Instant::now() - DEFAULT_TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );

        tokio::task::yield_now().await;
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

    #[tokio::test]
    async fn poll_task_uses_longer_timeout_before_first_output() {
        let entity = Entity::from_raw_u32(5).expect("valid entity id");
        let mut manager = make_manager();
        let mut handle = make_task_with_retries(ScriptedAttempt::PendingForever, Vec::new());
        handle.last_progress_at =
            Instant::now() - DEFAULT_TASK_NO_PROGRESS_TIMEOUT - Duration::from_secs(1);
        manager.tasks.insert(entity, handle);
        manager.results.insert(
            entity,
            make_result(TaskKind::Narration, TaskStatus::Running),
        );

        let status = manager.poll_task(entity);

        assert_eq!(status, TaskStatus::Running);
        assert_eq!(manager.results[&entity].status, TaskStatus::Running);
        assert!(manager.results[&entity].retry_history.is_empty());
    }
}
