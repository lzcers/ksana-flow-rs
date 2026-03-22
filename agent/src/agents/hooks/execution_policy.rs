use crate::agents::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleError, LifeCycleFlow, LifeCycleResult},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct ExecutionPolicyHook {
    iter_num: u32,
    max_iter_limit: u32,
}

impl ExecutionPolicyHook {
    pub fn new(max_iter_limit: u32) -> Self {
        Self {
            iter_num: 0,
            max_iter_limit,
        }
    }
    fn add_iter_num(&mut self) {
        self.iter_num += 1;
    }
    fn check_max_iter_limit(&self) -> bool {
        self.iter_num > self.max_iter_limit
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
    fn on(&self, stage: LifeCycle) -> bool {
        matches!(stage, LifeCycle::BeforeStep | LifeCycle::AfterStep)
    }
    async fn handle(mut self, ctx: &LifeCycleContext) -> LifeCycleFlow {
        match ctx.stage {
            LifeCycle::BeforeStep => {
                if self.check_max_iter_limit() {
                    return LifeCycleFlow::Break(LifeCycleError::hook_error(
                        &ctx.stage,
                        self.name(),
                        format!("max iter limit {} exceeded", self.max_iter_limit),
                    ));
                }
            }
            LifeCycle::AfterStep => self.add_iter_num(),
            _ => {}
        }
        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
