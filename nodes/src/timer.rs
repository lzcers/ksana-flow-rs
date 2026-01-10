use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use flow::{Context, Node};
use std::str::FromStr;
use tokio::time::sleep;
use tracing::info;

pub struct TimerNode {
    schedule: Schedule,
}

impl TimerNode {
    pub fn new(cron_expr: &str) -> Result<Self, cron::error::Error> {
        let schedule = Schedule::from_str(cron_expr)?;
        Ok(Self { schedule })
    }
}

#[async_trait]
impl Node for TimerNode {
    type In = ();
    type Out = ();

    async fn run(&mut self, _ctx: &Context, _input: Self::In) -> Self::Out {
        if let Some(next) = self.schedule.upcoming(Utc).next() {
            let now = Utc::now();
            if next > now {
                let duration = next - now;
                if let Ok(std_duration) = duration.to_std() {
                    sleep(std_duration).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_timer_node() {
        let ctx = Context::new();
        // 每秒执行一次的表达式 (7位: sec min hour day month dow year)
        let mut node = TimerNode::new("* * * * * * *").unwrap();

        let start = Instant::now();
        node.run(&ctx, ()).await;
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        // 应该至少等待到下一秒
        assert!(elapsed.as_millis() > 0);
    }
}
