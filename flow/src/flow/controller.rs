use crate::flow::runner::{FlowEvent, RunnerCommand};
use std::{future::Future, sync::Arc};
use tokio::sync::{broadcast, mpsc};

pub type ControllerHandle = Arc<Controller>;

pub struct Controller {
    cmd_tx: broadcast::Sender<RunnerCommand>,
    event_tx: mpsc::Sender<FlowEvent>,
}

impl Controller {
    pub fn new() -> (ControllerHandle, mpsc::Receiver<FlowEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (cmd_tx, _) = broadcast::channel(32);
        (
            Arc::new(Self { cmd_tx, event_tx }),
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
}

pub fn scope_controller<F, R>(controller: ControllerHandle, fut: F) -> impl Future<Output = R>
where
    F: Future<Output = R>,
{
    CONTROLLER.scope(controller, fut)
}

pub fn try_controller() -> Option<ControllerHandle> {
    CONTROLLER.try_with(|c| c.clone()).ok()
}

