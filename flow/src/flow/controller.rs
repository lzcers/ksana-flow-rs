use crate::NodeId;
use crate::flow::{
    graph::Graph,
    runner::{
        ExecutionContext, FlowEventEnvelope, Runner, RunnerCommand, RunnerHandle, SubgraphFrame,
    },
};
use dashmap::DashMap;
use std::sync::atomic::AtomicU16;
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
use tokio::{
    sync::{broadcast, mpsc},
    task::{AbortHandle, JoinHandle},
};

pub type ControllerHandle = Arc<Controller>;

pub type RunnerId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    Root,
    Subgraph,
}

pub struct RunnerRecord {
    pub id: RunnerId,
    pub parent: Option<RunnerId>,
    pub parent_node_id: Option<NodeId>,
    pub kind: RunnerKind,
    pub handle: RunnerHandle,
    abort: Mutex<Option<AbortHandle>>,
}

// Runner 的控制面，用于发送命令和接收事件
pub struct Controller {
    cmd_tx: broadcast::Sender<RunnerCommand>,
    event_tx: mpsc::Sender<FlowEventEnvelope>,
    next_runner_id: AtomicU16,
    runners: DashMap<RunnerId, Arc<RunnerRecord>>,
    max_concurrency: AtomicUsize,
}

impl Controller {
    pub fn new() -> (ControllerHandle, mpsc::Receiver<FlowEventEnvelope>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (cmd_tx, _) = broadcast::channel(32);
        (
            Arc::new(Self {
                cmd_tx,
                event_tx,
                next_runner_id: AtomicU16::new(1),
                runners: DashMap::new(),
                max_concurrency: AtomicUsize::new(0),
            }),
            event_rx,
        )
    }

    pub fn cmd_tx(&self) -> broadcast::Sender<RunnerCommand> {
        self.cmd_tx.clone()
    }
    pub fn get_flow_event_sender(&self) -> mpsc::Sender<FlowEventEnvelope> {
        self.event_tx.clone()
    }
    pub fn set_max_concurrency(&self, max: usize) {
        if max == 0 {
            self.clear_max_concurrency();
            return;
        }
        self.max_concurrency.store(max, Ordering::Relaxed);
        let _ = self.cmd_tx.send(RunnerCommand::SetMaxConcurrency(max));
    }

    pub fn clear_max_concurrency(&self) {
        self.max_concurrency.store(0, Ordering::Relaxed);
        let _ = self.cmd_tx.send(RunnerCommand::ClearMaxConcurrency);
    }

    fn get_max_concurrency_snapshot(&self) -> Option<usize> {
        let v = self.max_concurrency.load(Ordering::Relaxed);
        if v == 0 { None } else { Some(v) }
    }

    pub fn describe_runner(
        &self,
        runner_id: RunnerId,
    ) -> (
        RunnerKind,
        Option<RunnerId>,
        Option<NodeId>,
        Vec<SubgraphFrame>,
    ) {
        let mut kind = RunnerKind::Root;
        let mut parent_runner_id = None;
        let mut parent_node_id = None;
        let mut frames = Vec::new();

        let mut current = Some(runner_id);
        while let Some(id) = current {
            let record = match self.runners.get(&id) {
                Some(r) => r,
                None => break,
            };

            if id == runner_id {
                kind = record.kind;
                parent_runner_id = record.parent;
                parent_node_id = record.parent_node_id.clone();
            }

            if record.kind == RunnerKind::Subgraph {
                if let Some(node_id) = record.parent_node_id.clone() {
                    frames.push(SubgraphFrame {
                        runner_id: record.id,
                        parent_node_id: node_id,
                    });
                }
            }

            current = record.parent;
        }

        frames.reverse();
        (kind, parent_runner_id, parent_node_id, frames)
    }
}

tokio::task_local! {
    static CONTROLLER: ControllerHandle;
    static RUNNER_ID: RunnerId;
    static CURRENT_NODE_ID: NodeId;
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

pub fn scope_current_node<F, R>(node_id: NodeId, fut: F) -> impl Future<Output = R>
where
    F: Future<Output = R>,
{
    CURRENT_NODE_ID.scope(node_id, fut)
}

pub fn try_controller() -> Option<ControllerHandle> {
    CONTROLLER.try_with(|c| c.clone()).ok()
}

pub fn try_runner_id() -> Option<RunnerId> {
    RUNNER_ID.try_with(|id| *id).ok()
}

pub fn try_current_node_id() -> Option<NodeId> {
    CURRENT_NODE_ID.try_with(|id| id.clone()).ok()
}

pub trait ControllerRunners {
    fn create_runner(
        &self,
        graph: Arc<Graph>,
        initial: Option<ExecutionContext>,
        kind: RunnerKind,
        parent: Option<RunnerId>,
        parent_node_id: Option<NodeId>,
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
        parent_node_id: Option<NodeId>,
    ) -> (RunnerId, Runner, RunnerHandle) {
        let runner_id = self.next_runner_id.fetch_add(1, Ordering::Relaxed);
        let (mut runner, handle) = Runner::new(graph, initial, self.clone(), runner_id);
        if let Some(max) = self.get_max_concurrency_snapshot() {
            runner.apply_max_concurrency(max);
        }
        let record = Arc::new(RunnerRecord {
            id: runner_id,
            parent,
            parent_node_id,
            kind,
            handle: handle.clone(),
            abort: Mutex::new(None),
        });
        self.runners.insert(runner_id, record);
        (runner_id, runner, handle)
    }

    // runner 注册与执行分离
    // 从而允许更灵活的控制资源与作用域，方便将 controller_for_scope 与 runner_id 绑定
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

    // 强制终止 runner，不等待 runner 事件通道关闭
    // 主要用于处理异常情况，如 runner 死锁、panic 等
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

    // 通过 cmd 发送 Stop 命令，停止所有 runner
    fn stop_all(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Stop);
    }

    fn get_runner_handle(&self, runner_id: RunnerId) -> Option<RunnerHandle> {
        self.runners.get(&runner_id).map(|r| r.handle.clone())
    }
}
