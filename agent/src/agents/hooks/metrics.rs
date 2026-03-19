use std::{sync::Mutex, time::Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AfterStep, HookError, HookOutcome, RuntimeHook, StepHookContext};

const ACTIVE_STEP_STARTED_AT_KEY: &str = "metrics.active_step_started_at";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionMetrics {
    pub total_duration: std::time::Duration,
    pub tool_calls_count: usize,
    pub iterations: usize,
}

#[derive(Debug, Default)]
struct MetricsState {
    metrics: ExecutionMetrics,
    active_step_started_at: Option<Instant>,
}

#[derive(Default)]
pub struct MetricsHook {
    state: Mutex<MetricsState>,
}

impl MetricsHook {
    pub fn metrics(&self) -> ExecutionMetrics {
        self.state.lock().unwrap().metrics.clone()
    }
}

#[async_trait]
impl RuntimeHook for MetricsHook {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn snapshot(&self) -> Option<Value> {
        serde_json::to_value(self.metrics()).ok()
    }

    async fn before_step(&self, ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError> {
        let started_at = Instant::now();
        {
            let mut state = self.state.lock().unwrap();
            state.metrics.iterations = ctx.state.iteration;
            state.active_step_started_at = Some(started_at);
        }
        ctx.scratchpad
            .insert(ACTIVE_STEP_STARTED_AT_KEY, started_at);
        Ok(HookOutcome::Continue)
    }

    async fn after_call_tools(
        &self,
        _ctx: &mut StepHookContext<'_>,
        input: &mut super::AfterCallTools<'_>,
    ) -> Result<HookOutcome, HookError> {
        let mut state = self.state.lock().unwrap();
        state.metrics.tool_calls_count += input.tool_calls.len();
        Ok(HookOutcome::Continue)
    }

    async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        _input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        let started_at = ctx
            .scratchpad
            .get::<Instant>(ACTIVE_STEP_STARTED_AT_KEY)
            .copied()
            .or_else(|| self.state.lock().unwrap().active_step_started_at);

        if let Some(started_at) = started_at {
            let mut state = self.state.lock().unwrap();
            state.metrics.total_duration += started_at.elapsed();
            state.active_step_started_at = None;
        }

        Ok(HookOutcome::Continue)
    }
}
