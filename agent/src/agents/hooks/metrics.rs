use std::{sync::Mutex, time::Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AfterCallTools, AfterStep, BeforeStep, Effect, HookError, RuntimeHook};

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

    async fn before_step(&self, input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        let started_at = Instant::now();
        {
            let mut state = self.state.lock().unwrap();
            state.metrics.iterations = input.state.iteration;
            state.active_step_started_at = Some(started_at);
        }
        Ok(vec![Effect::store_scratchpad(
            ACTIVE_STEP_STARTED_AT_KEY,
            started_at,
        )])
    }

    async fn after_call_tools(&self, input: AfterCallTools<'_>) -> Result<Vec<Effect>, HookError> {
        let mut state = self.state.lock().unwrap();
        state.metrics.tool_calls_count += input.tool_results.len();
        Ok(vec![])
    }

    async fn after_step(&self, input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        match input.result {
            super::StepResultDraft::Continue { .. }
            | super::StepResultDraft::Done { .. }
            | super::StepResultDraft::Error(_) => {}
        }

        let started_at = input
            .frame
            .scratchpad
            .get::<Instant>(ACTIVE_STEP_STARTED_AT_KEY)
            .copied()
            .or_else(|| self.state.lock().unwrap().active_step_started_at);

        if let Some(started_at) = started_at {
            let mut state = self.state.lock().unwrap();
            state.metrics.total_duration += started_at.elapsed();
            state.active_step_started_at = None;
        }

        Ok(vec![])
    }
}
