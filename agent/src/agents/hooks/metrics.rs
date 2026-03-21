use crate::agents::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleEffect, LifeCycleError, LifeCycleFlow},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct MetricsHook {}

impl MetricsHook {
    pub fn new(max_iter_limit: u32) -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for MetricsHook {
    fn name(&self) -> HookName {
        "metrics"
    }
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: LifeCycle) -> bool {
        todo!("on")
    }
    async fn handle(mut self, ctx: &LifeCycleContext) -> LifeCycleFlow {
        todo!()
    }
}
