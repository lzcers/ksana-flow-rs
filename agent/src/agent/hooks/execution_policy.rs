use crate::agent::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow, LifeCycleInterrupt, LifeCycleResult},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct ExecutionPolicyHook;

impl ExecutionPolicyHook {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for ExecutionPolicyHook {
    fn name(&self) -> HookName {
        "execution_policy"
    }
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: &LifeCycle) -> bool {
        matches!(stage, LifeCycle::BeforeStep)
    }

    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow {
        if matches!(ctx.stage, LifeCycle::BeforeStep) {
            let iteration = ctx.state.metrics.execution.iteration;
            let max_iter_limit = ctx.state.metrics.execution.max_iterations;

            if iteration >= max_iter_limit {
                return LifeCycleFlow::Break(LifeCycleInterrupt::hook_error(
                    &ctx.stage,
                    self.name(),
                    format!("max iter limit {} exceeded", max_iter_limit),
                ));
            }
        }
        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
