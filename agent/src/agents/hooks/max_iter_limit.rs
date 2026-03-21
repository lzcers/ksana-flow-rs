use crate::agents::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleEffect, LifeCycleError, LifeCycleFlow},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct MaxIterLimitHook {
    iter_num: u32,
    max_iter_limit: u32,
}

impl MaxIterLimitHook {
    pub fn new(max_iter_limit: u32) -> Self {
        Self {
            iter_num: 0,
            max_iter_limit,
        }
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for MaxIterLimitHook {
    fn name(&self) -> HookName {
        "max_iter_limit"
    }
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: LifeCycle) -> bool {
        matches!(stage, LifeCycle::BeforeStep | LifeCycle::AfterStep)
    }
    async fn handle(mut self, ctx: &LifeCycleContext) -> LifeCycleFlow {
        if ctx.stage == LifeCycle::BeforeStep && self.iter_num > self.max_iter_limit {
            LifeCycleFlow::Break(LifeCycleError::hook_error(
                &ctx.stage,
                self.name(),
                format!("max iter limit {} exceeded", self.max_iter_limit),
            ))
        } else if ctx.stage == LifeCycle::AfterStep {
            self.iter_num += 1;
            LifeCycleFlow::Continue(LifeCycleEffect::None)
        } else {
            LifeCycleFlow::Continue(LifeCycleEffect::None)
        }
    }
}
