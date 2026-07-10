use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use flow::{Context, Input, Node, Output, ReactiveStream};
use futures::stream;
use std::str::FromStr;
use tokio::time::sleep;

pub struct TimerNode {
    cron_expr: String,
}

impl TimerNode {
    pub fn new(cron_expr: &str) -> Result<Self, cron::error::Error> {
        Ok(Self {
            cron_expr: cron_expr.to_string(),
        })
    }
}

fn timer_stream(schedule: Schedule) -> impl futures::Stream<Item = Result<(), ()>> + Send {
    stream::unfold(schedule, |schedule| async move {
        if let Some(next) = schedule.upcoming(Utc).next() {
            let now = Utc::now();
            if next > now {
                let duration = next - now;
                if let Ok(std_duration) = duration.to_std() {
                    sleep(std_duration).await;
                }
            }
            Some((Ok(()), schedule))
        } else {
            None
        }
    })
}

#[async_trait]
impl Node for TimerNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let schedule = Schedule::from_str(&self.cron_expr).expect("Invalid cron expression");
        let stream = ReactiveStream::from_stream(timer_stream(schedule));
        let mut out = Output::new(None);
        out.set_stream(stream);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::TaskEvent;
    use flow::TaskGuard;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_timer_node() {
        let ctx = Context::new();
        // Execute every second (7 fields: sec min hour day month dow year)
        let mut node = TimerNode::new("* * * * * * *").unwrap();

        let out = node
            .run(&ctx, &Input::new(HashMap::<String, Value>::new()))
            .await
            .unwrap();
        let stream = out.into_stream().expect("Expected stream output");
        let start_stream = stream.start;

        let (tx, mut rx) = mpsc::channel(10);

        // Manually start the stream
        let task = start_stream(
            TaskGuard::default(),
            tx,
            "test_node".to_string(),
            Arc::new(ctx),
        );

        // Wait for the first event
        if let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Next(node_id, val) => {
                    assert_eq!(node_id, "test_node");
                    assert_eq!(val, Value::Null);
                }
                _ => panic!("Expected Next event"),
            }
        }

        task.abort();
        while rx.try_recv().is_ok() {}
        assert!(
            timeout(Duration::from_millis(100), rx.recv())
                .await
                .expect("aborted timer should close its event channel")
                .is_none()
        );
    }
}
