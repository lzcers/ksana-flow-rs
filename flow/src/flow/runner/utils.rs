use serde_json::Value;
use tokio::sync::mpsc;

use crate::{FlowEvent, SendableAny, TaskEvent};

pub fn flatten_sendable_any(mut out: Box<dyn SendableAny>) -> Box<dyn SendableAny> {
    while out.as_any().is::<Box<dyn SendableAny>>() {
        let any = out.into_any();
        match any.downcast::<Box<dyn SendableAny>>() {
            Ok(inner) => {
                out = *inner;
            }
            Err(orig) => {
                // Handle edge case where type identification mismatches
                let orig = match orig.downcast::<String>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<Box<String>>() {
                    Ok(s) => return Box::new(**s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<i32>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<bool>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<f64>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<Value>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<()>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };
                let orig = match orig.downcast::<(i32, bool)>() {
                    Ok(s) => return Box::new(*s),
                    Err(f) => f,
                };

                // Try other common types if needed, or panic
                panic!("Failed to flatten SendableAny: {:?}", orig.type_id());
            }
        }
    }
    out
}

pub async fn send_flow_event(sender: &Option<mpsc::Sender<FlowEvent>>, event: FlowEvent) {
    if let Some(sender) = sender {
        let _ = sender.send(event).await;
    }
}

pub async fn send_task_event(sender: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
    let _ = sender.send(event).await;
}
