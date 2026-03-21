pub mod max_iter_limit;
pub mod metrics;
pub mod token_statistics;
use crate::agents::agent_actor::lifecycle::{LifeCycle, LifeCycleContext, LifeCycleFlow};

pub type HookName = &'static str;
#[async_trait::async_trait]
pub trait LifeCycleHook: Send + Sync {
    fn name(&self) -> HookName;
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: LifeCycle) -> bool {
        matches!(stage, LifeCycle::BeforeStep | LifeCycle::AfterStep)
    }

    async fn handle(mut self, ctx: &LifeCycleContext) -> LifeCycleFlow;
}
