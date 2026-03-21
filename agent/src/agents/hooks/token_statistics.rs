use crate::agents::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct TokenStatisticsHook {}

impl TokenStatisticsHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for TokenStatisticsHook {
    fn name(&self) -> HookName {
        "token_statistics"
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
