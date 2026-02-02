use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use flow::observable::{Observable, Observer, Subscription};
use flow::{Context, Input, Node, Output, ReactiveStream};
use serde_json::Value;
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

pub struct TimerObservable {
    schedule: Schedule,
}

pub struct TimerSubscription;

impl Subscription for TimerSubscription {
    fn unsubscribe(self: Box<Self>) {}
}

#[async_trait]
impl Observable<(), ()> for TimerObservable {
    type Sub = TimerSubscription;

    async fn subscribe(self, mut observer: impl Observer<(), ()> + 'static) -> TimerSubscription {
        loop {
            // Find the next scheduled time
            if let Some(next) = self.schedule.upcoming(Utc).next() {
                let now = Utc::now();
                if next > now {
                    let duration = next - now;
                    if let Ok(std_duration) = duration.to_std() {
                        sleep(std_duration).await;
                    }
                }
                // Emit event
                observer.on_next(()).await;
            } else {
                // Should not happen for cron, but if it does, complete
                observer.on_completed().await;
                break;
            }
        }
        TimerSubscription
    }
}

#[async_trait]
impl Node for TimerNode {
    async fn run(
        &mut self,
        _ctx: &Context,
        _input: &Input,
    ) -> Result<Output, String> {
        let schedule = Schedule::from_str(&self.cron_expr).expect("Invalid cron expression");
        let observable = TimerObservable { schedule };
        let stream = ReactiveStream::from_observable(observable);
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
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_timer_node() {
        let ctx = Context::new();
        // Execute every second (7 fields: sec min hour day month dow year)
        let mut node = TimerNode::new("* * * * * * *").unwrap();

        let start = Instant::now();
        let out = node.run(&ctx, &Input::new(HashMap::<String, Value>::new()))
            .await
            .unwrap();
        let stream = out.into_stream().expect("Expected stream output");
        let subscribe = stream.subscribe;

        let (tx, mut rx) = mpsc::channel(10);

        // Manually subscribe to the stream
        let sub = subscribe(
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

        sub.unsubscribe();
        while rx.try_recv().is_ok() {}
        let deadline = tokio::time::Instant::now() + Duration::from_millis(2500);
        let mut received_after_unsub = 0usize;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline - now;
            match timeout(remaining, rx.recv()).await {
                Ok(Some(_)) => {
                    received_after_unsub += 1;
                    if received_after_unsub > 1 {
                        panic!("Received too many events after unsubscribe");
                    }
                }
                _ => break,
            }
        }

        let elapsed = start.elapsed();
        println!("Elapsed: {:?}", elapsed);
    }
}
