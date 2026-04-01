use tokio::time::Instant;

use crate::agent::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow, LifeCycleResult},
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
        self.step_call_model_duration = 0;
        self.step_call_model_instant = Some(Instant::now());
    }
    fn record_step_call_model_duration(&mut self, ctx: &mut LifeCycleContext) {
        if let Some(inst) = self.step_call_model_instant {
            self.step_call_model_duration = inst.elapsed().as_millis() as u32;
            ctx.set_frame_call_model_duration_ms(self.step_call_model_duration);
        }
    }
    fn set_step_call_tools_instant(&mut self) {
        self.step_tools_call_duration = 0;
        self.step_call_tools_instant = Some(Instant::now());
    }
    fn record_step_call_tools_duration(&mut self, ctx: &mut LifeCycleContext) {
        if let Some(inst) = self.step_call_tools_instant {
            self.step_tools_call_duration = inst.elapsed().as_millis() as u32;
            ctx.set_frame_call_tools_duration_ms(self.step_tools_call_duration);
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
    fn on(&self, stage: &LifeCycle) -> bool {
        matches!(
            stage,
            LifeCycle::BeforeCallModel
                | LifeCycle::AfterCallModel
                | LifeCycle::BeforeCallTools
                | LifeCycle::AfterCallTools
        )
    }
    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow {
        match ctx.stage {
            LifeCycle::BeforeCallModel => self.set_step_call_model_instant(),
            LifeCycle::AfterCallModel => self.record_step_call_model_duration(ctx),
            LifeCycle::BeforeCallTools => self.set_step_call_tools_instant(),
            LifeCycle::AfterCallTools => self.record_step_call_tools_duration(ctx),
            _ => {}
        }

        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
