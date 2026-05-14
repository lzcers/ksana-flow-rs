use std::sync::RwLock;

use bevy_ecs::resource::Resource;
use serde::Serialize;
use tokio::sync::{broadcast, watch};

use crate::resources::{
    task_manager::{TaskKind, TaskResult, TaskStatus, TaskUpdate},
    turn_state::{TurnPhase, TurnState},
    world_snapshot::WorldSnapshot,
};

const DEFAULT_EXPORT_EVENT_BUFFER: usize = 256;

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub turn: TurnView,
    pub world: WorldSnapshot,
    pub tasks: Vec<TaskView>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TurnView {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
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
pub enum SessionEvent {
    TurnChanged { turn: TurnView },
    WorldSnapshotUpdated { world: WorldSnapshot },
    TaskUpdated { task: TaskView, update: TaskUpdate },
}

#[derive(Resource)]
pub struct ExportState {
    latest_snapshot: RwLock<SessionSnapshot>,
    snapshot_tx: watch::Sender<SessionSnapshot>,
    event_tx: broadcast::Sender<SessionEvent>,
}

#[derive(Clone)]
pub struct ExportHandle {
    snapshot_rx: watch::Receiver<SessionSnapshot>,
    event_tx: broadcast::Sender<SessionEvent>,
}

impl ExportState {
    pub fn new() -> Self {
        Self::with_buffer(DEFAULT_EXPORT_EVENT_BUFFER)
    }

    pub fn new_with_handle() -> (Self, ExportHandle) {
        let state = Self::new();
        let handle = state.handle();
        (state, handle)
    }

    pub fn with_buffer(event_buffer: usize) -> Self {
        let initial = SessionSnapshot::default();
        let (snapshot_tx, _) = watch::channel(initial.clone());
        let (event_tx, _) = broadcast::channel(event_buffer);

        Self {
            latest_snapshot: RwLock::new(initial),
            snapshot_tx,
            event_tx,
        }
    }

    pub fn handle(&self) -> ExportHandle {
        ExportHandle {
            snapshot_rx: self.snapshot_tx.subscribe(),
            event_tx: self.event_tx.clone(),
        }
    }

    pub fn publish_snapshot(&self, snapshot: SessionSnapshot) {
        let (turn_changed, world_changed) = {
            let current = self
                .latest_snapshot
                .read()
                .expect("export snapshot lock poisoned");
            (
                current.turn != snapshot.turn,
                current.world != snapshot.world,
            )
        };

        {
            let mut current = self
                .latest_snapshot
                .write()
                .expect("export snapshot lock poisoned");
            *current = snapshot.clone();
        }

        let _ = self.snapshot_tx.send(snapshot.clone());
        if turn_changed {
            let _ = self.event_tx.send(SessionEvent::TurnChanged {
                turn: snapshot.turn,
            });
        }
        if world_changed {
            let _ = self.event_tx.send(SessionEvent::WorldSnapshotUpdated {
                world: snapshot.world,
            });
        }
    }

    pub fn publish_task_update(&self, task: TaskView, update: TaskUpdate) {
        let _ = self
            .event_tx
            .send(SessionEvent::TaskUpdated { task, update });
    }

    pub fn current_snapshot(&self) -> SessionSnapshot {
        self.latest_snapshot
            .read()
            .expect("export snapshot lock poisoned")
            .clone()
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

    pub fn subscribe_events(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }
}

impl TurnView {
    pub fn from_turn_state(turn_state: &TurnState) -> Self {
        Self {
            phase: turn_state.phase,
            turn_index: turn_state.turn_index,
            active_turn_id: turn_state.active_turn_id,
        }
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
