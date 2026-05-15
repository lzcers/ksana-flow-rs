pub mod ask_user;
pub mod execution_policy;
pub mod metrics;
pub mod send_model_evt;
pub mod update_frame;

use crate::agent::agent_actor::lifecycle::{LifeCycle, LifeCycleContext, LifeCycleFlow};

pub type HookName = &'static str;
#[async_trait::async_trait]
pub trait LifeCycleHook: Send + Sync {
    fn name(&self) -> HookName;
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: &LifeCycle) -> bool;

    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow;
}
