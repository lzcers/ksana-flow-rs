use std::time::Duration;

use serde_json::{Value, json};

use super::{AgentHook, ExecutionPolicy};
use crate::agents::AgentState;

#[derive(Debug, Clone, Default)]
pub struct TimeoutPolicyHook {
    step_timeout: Option<Duration>,
    tool_timeout: Option<Duration>,
}

impl TimeoutPolicyHook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = Some(timeout);
        self
    }

    pub fn tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = Some(timeout);
        self
    }
}

impl AgentHook for TimeoutPolicyHook {
    fn name(&self) -> &'static str {
        "timeout_policy"
    }

    fn configure_execution_policy(&self, _state: &AgentState, policy: &mut ExecutionPolicy) {
        if let Some(step_timeout) = self.step_timeout {
            policy.step_timeout = Some(step_timeout);
        }
        if let Some(tool_timeout) = self.tool_timeout {
            policy.tool_timeout = Some(tool_timeout);
        }
    }

    fn snapshot(&self) -> Option<Value> {
        Some(json!({
            "step_timeout_ms": self.step_timeout.map(|timeout| timeout.as_millis() as u64),
            "tool_timeout_ms": self.tool_timeout.map(|timeout| timeout.as_millis() as u64),
        }))
    }
}
