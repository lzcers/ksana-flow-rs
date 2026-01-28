use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use flow::observable::{Observable, Observer, Subscription};
use flow::{Context, Node, NodeInputs, OutputPayload, ReactiveStream};
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
    fn unsubscribe(self) {}
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
    async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> Result<OutputPayload, String> {
        let schedule = Schedule::from_str(&self.cron_expr).expect("Invalid cron expression");
        let observable = TimerObservable { schedule };
        let stream = ReactiveStream::from_observable(observable);
        Ok(OutputPayload::stream(stream.subscribe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::NodeInputs;
    use flow::OutputPayload;
    use flow::TaskEvent;
    use flow::TaskGuard;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_timer_node() {
        let ctx = Context::new();
        // Execute every second (7 fields: sec min hour day month dow year)
        let mut node = TimerNode::new("* * * * * * *").unwrap();

        let start = Instant::now();
        let payload = node
            .run(&ctx, NodeInputs::new(HashMap::new()))
            .await
            .unwrap();
        let OutputPayload::Stream(subscribe) = payload else {
            panic!("Expected stream payload");
        };

        let (tx, mut rx) = mpsc::channel(10);

        // Manually subscribe to the stream
        subscribe(
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
                    // Verify the value is ()
                    assert!(val.as_any().expect("Expected value").is::<()>());
                }
                _ => panic!("Expected Next event"),
            }
        }

        let elapsed = start.elapsed();
        println!("Elapsed: {:?}", elapsed);
    }
}
