use bevy_ecs::resource::Resource;
use serde::Serialize;
use tokio::sync::{broadcast, watch};

use crate::resources::{
    task_manager::{TaskKind, TaskResult, TaskStatus},
    turn_state::TurnPhase,
    world_snapshot::WorldSnapshot,
};

const DEFAULT_EXPORT_EVENT_BUFFER: usize = 256;

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub world: WorldSnapshot,
    pub tasks: Vec<TaskView>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub entity: String,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    TaskUpdated { task: TaskView },
}

#[derive(Resource)]
pub struct ExportState {
    snapshot_tx: watch::Sender<SessionSnapshot>,
    event_tx: broadcast::Sender<TaskEvent>,
}

#[derive(Clone)]
pub struct ExportHandle {
    snapshot_rx: watch::Receiver<SessionSnapshot>,
    event_tx: broadcast::Sender<TaskEvent>,
}

impl ExportState {
    pub fn new() -> Self {
        Self::with_buffer(DEFAULT_EXPORT_EVENT_BUFFER)
    }

    pub fn new_with_handle() -> (Self, ExportHandle) {
        let state = Self::new();
        let handle = ExportHandle {
            snapshot_rx: state.snapshot_tx.subscribe(),
            event_tx: state.event_tx.clone(),
        };
        (state, handle)
    }

    pub fn with_buffer(event_buffer: usize) -> Self {
        let initial = SessionSnapshot::default();
        let (snapshot_tx, _) = watch::channel(initial);
        let (event_tx, _) = broadcast::channel(event_buffer);

        Self {
            snapshot_tx,
            event_tx,
        }
    }

    pub fn publish_snapshot(&self, snapshot: SessionSnapshot) {
        self.snapshot_tx.send_replace(snapshot);
    }

    pub fn publish_task_update(&self, task: TaskView) {
        let _ = self.event_tx.send(TaskEvent::TaskUpdated { task });
    }

    pub fn current_snapshot(&self) -> SessionSnapshot {
        self.snapshot_tx.borrow().clone()
    }
}

impl Default for ExportState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportHandle {
    pub fn current_snapshot(&self) -> SessionSnapshot {
        self.snapshot_rx.borrow().clone()
    }

    pub fn snapshot_receiver(&self) -> watch::Receiver<SessionSnapshot> {
        self.snapshot_rx.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_tx.subscribe()
    }
}

impl TaskView {
    pub fn from_task_result(entity: String, result: TaskResult) -> Self {
        let output = result
            .result
            .as_ref()
            .and_then(|value| value.as_ref().ok().cloned());
        let error = result
            .result
            .as_ref()
            .and_then(|value| value.as_ref().err().cloned())
            .or_else(|| result.last_error.clone());

        Self {
            entity,
            kind: result.kind,
            status: result.status,
            attempts: result.attempts,
            max_attempts: result.max_attempts,
            last_error: result.last_error,
            chunks: result.chunks,
            output,
            error,
        }
    }
}
