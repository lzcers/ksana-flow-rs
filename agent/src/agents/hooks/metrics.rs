use tokio::time::Instant;

use crate::agents::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleEffect, LifeCycleError, LifeCycleFlow},
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct MetricsHook {
    step_call_model_duration: u32,
    step_tools_call_duration: u32,
    step_call_model_instant: Option<Instant>,
    step_call_tools_instant: Option<Instant>,
}

impl MetricsHook {
    pub fn new() -> Self {
        Self {
            step_call_model_duration: 0,
            step_tools_call_duration: 0,
            step_call_model_instant: None,
            step_call_tools_instant: None,
        }
    }
}

impl MetricsHook {
    fn set_step_call_model_instant(&mut self) {
        self.step_call_model_instant = Some(Instant::now());
    }
    fn record_step_call_model_duration(&mut self) {
        if let Some(inst) = self.step_call_model_instant {
            self.step_call_model_duration = inst.elapsed().as_millis() as u32;
        }
    }
    fn set_step_call_tools_instant(&mut self) {
        self.step_call_tools_instant = Some(Instant::now());
    }
    fn record_step_call_tools_duration(&mut self) {
        if let Some(inst) = self.step_call_tools_instant {
            self.step_tools_call_duration = inst.elapsed().as_millis() as u32;
        }
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
        matches!(
            stage,
            LifeCycle::BeforeCallModel
                | LifeCycle::AfterCallModel
                | LifeCycle::BeforeCallTools
                | LifeCycle::AfterCallTools
        )
    }
    async fn handle(mut self, ctx: &LifeCycleContext) -> LifeCycleFlow {
        match ctx.stage {
            LifeCycle::BeforeCallModel => self.set_step_call_model_instant(),
            LifeCycle::AfterCallModel => self.record_step_call_model_duration(),
            LifeCycle::BeforeCallTools => self.set_step_call_tools_instant(),
            LifeCycle::AfterCallTools => self.record_step_call_tools_duration(),
            _ => {}
        }

        LifeCycleFlow::Continue(LifeCycleEffect::None)
    }
}
