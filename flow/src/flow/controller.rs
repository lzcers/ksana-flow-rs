use crate::flow::{
    graph::Graph,
    runner::{ExecutionContext, FlowEvent, Runner, RunnerCommand, RunnerHandle},
};
use dashmap::DashMap;
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{
    sync::{broadcast, mpsc},
    task::{AbortHandle, JoinHandle},
};

pub type ControllerHandle = Arc<Controller>;

pub type RunnerId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    Root,
    Subgraph,
}

pub struct RunnerRecord {
    pub id: RunnerId,
    pub parent: Option<RunnerId>,
    pub kind: RunnerKind,
    pub handle: RunnerHandle,
    abort: Mutex<Option<AbortHandle>>,
}

pub struct Controller {
    cmd_tx: broadcast::Sender<RunnerCommand>,
    event_tx: mpsc::Sender<FlowEvent>,
    next_runner_id: AtomicU64,
    runners: DashMap<RunnerId, Arc<RunnerRecord>>,
}

impl Controller {
    pub fn new() -> (ControllerHandle, mpsc::Receiver<FlowEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (cmd_tx, _) = broadcast::channel(32);
        (
            Arc::new(Self {
                cmd_tx,
                event_tx,
                next_runner_id: AtomicU64::new(1),
                runners: DashMap::new(),
            }),
            event_rx,
        )
    }

    pub fn cmd_tx(&self) -> broadcast::Sender<RunnerCommand> {
        self.cmd_tx.clone()
    }

    pub fn cmd_rx(&self) -> broadcast::Receiver<RunnerCommand> {
        self.cmd_tx.subscribe()
    }

    pub fn event_tx(&self) -> mpsc::Sender<FlowEvent> {
        self.event_tx.clone()
    }

    pub async fn send_event(&self, event: FlowEvent) {
        let _ = self.event_tx.send(event).await;
    }
}

tokio::task_local! {
    static CONTROLLER: ControllerHandle;
    static RUNNER_ID: RunnerId;
}

pub fn scope_controller<F, R>(controller: ControllerHandle, fut: F) -> impl Future<Output = R>
where
    F: Future<Output = R>,
{
    CONTROLLER.scope(controller, fut)
}

pub fn scope_runner<F, R>(
    controller: ControllerHandle,
    runner_id: RunnerId,
    fut: F,
) -> impl Future<Output = R>
where
    F: Future<Output = R>,
{
    CONTROLLER.scope(controller, RUNNER_ID.scope(runner_id, fut))
}

pub fn try_controller() -> Option<ControllerHandle> {
    CONTROLLER.try_with(|c| c.clone()).ok()
}

pub fn try_runner_id() -> Option<RunnerId> {
    RUNNER_ID.try_with(|id| *id).ok()
}

pub trait ControllerRunners {
    fn create_runner(
        &self,
        graph: Arc<Graph>,
        initial: Option<ExecutionContext>,
        kind: RunnerKind,
        parent: Option<RunnerId>,
    ) -> (RunnerId, Runner, RunnerHandle);

    fn spawn_runner(&self, runner_id: RunnerId, runner: Runner) -> JoinHandle<Result<(), String>>;

    fn abort_runner(&self, runner_id: RunnerId) -> bool;

    fn unregister_runner(&self, runner_id: RunnerId) -> bool;

    fn stop_all(&self);

    fn get_runner_handle(&self, runner_id: RunnerId) -> Option<RunnerHandle>;
}

impl ControllerRunners for ControllerHandle {
    fn create_runner(
        &self,
        graph: Arc<Graph>,
        initial: Option<ExecutionContext>,
        kind: RunnerKind,
        parent: Option<RunnerId>,
    ) -> (RunnerId, Runner, RunnerHandle) {
        let runner_id = self.next_runner_id.fetch_add(1, Ordering::Relaxed);
        let (runner, handle) = Runner::new(graph, initial, self.clone(), runner_id);
        let record = Arc::new(RunnerRecord {
            id: runner_id,
            parent,
            kind,
            handle: handle.clone(),
            abort: Mutex::new(None),
        });
        self.runners.insert(runner_id, record);
        (runner_id, runner, handle)
    }

    fn spawn_runner(&self, runner_id: RunnerId, runner: Runner) -> JoinHandle<Result<(), String>> {
        let controller_for_scope = self.clone();
        let controller_for_cleanup = self.clone();
        let task: JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let mut runner = runner;
            let res = scope_runner(controller_for_scope, runner_id, async move {
                runner.run().await
            })
            .await;
            controller_for_cleanup.runners.remove(&runner_id);
            res
        });

        if let Some(record) = self.runners.get(&runner_id).map(|v| v.clone()) {
            if let Ok(mut guard) = record.abort.lock() {
                *guard = Some(task.abort_handle());
            }
        }

        task
    }

    fn abort_runner(&self, runner_id: RunnerId) -> bool {
        let record = self.runners.remove(&runner_id).map(|(_, v)| v);
        if let Some(record) = record {
            if let Ok(mut guard) = record.abort.lock() {
                if let Some(abort) = guard.take() {
                    abort.abort();
                }
            }
            true
        } else {
            false
        }
    }

    fn unregister_runner(&self, runner_id: RunnerId) -> bool {
        self.runners.remove(&runner_id).is_some()
    }

    fn stop_all(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Stop);
        let runner_ids: Vec<RunnerId> = self.runners.iter().map(|r| *r.key()).collect();
        for runner_id in runner_ids {
            self.abort_runner(runner_id);
        }
    }

    fn get_runner_handle(&self, runner_id: RunnerId) -> Option<RunnerHandle> {
        self.runners.get(&runner_id).map(|r| r.handle.clone())
    }
}
