use tokio::sync::mpsc;

use crate::{FlowEvent, TaskEvent};

pub async fn send_flow_event(sender: &Option<mpsc::Sender<FlowEvent>>, event: FlowEvent) {
    if let Some(sender) = sender {
        let _ = sender.send(event).await;
    }
}

pub async fn send_task_event(sender: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
    let _ = sender.send(event).await;
}
