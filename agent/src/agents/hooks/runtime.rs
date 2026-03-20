use std::{any::Any, collections::HashMap, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use super::{Effect, StepFrame};
use crate::agents::{AgentError, AgentState, CallModelEvent, CallToolResult, ToolCall};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPhase {
    BeforeStep,
    BeforeCallModel,
    OnModelEvent,
    AfterCallModel,
    BeforeCallTools,
    AfterCallTools,
    AfterStep,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct HookError {
    pub message: String,
}

impl HookError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum StepResultDraft {
    Continue {
        content: String,
        reasoning_content: Option<String>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<CallToolResult>,
    },
    Done {
        content: String,
        reasoning_content: Option<String>,
    },
    Error(AgentError),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExecutionPolicy {
    pub step_timeout: Option<Duration>,
    pub tool_timeout: Option<Duration>,
}

#[derive(Debug, Default)]
pub(crate) struct StepScratchpad {
    inner: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl StepScratchpad {
    pub(crate) fn insert_box(&mut self, key: &'static str, value: Box<dyn Any + Send + Sync>) {
        self.inner.insert(key, value);
    }

    pub(crate) fn get<T: Send + Sync + 'static>(&self, key: &'static str) -> Option<&T> {
        self.inner.get(key)?.downcast_ref::<T>()
    }
}

pub(crate) struct BeforeStep<'a> {
    pub state: &'a AgentState,
}

pub(crate) struct ModelEventCtx<'a> {
    pub event: &'a CallModelEvent,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ModelCallOutput {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

impl ModelCallOutput {
    pub(crate) fn tool_calls_option(&self) -> Option<Vec<ToolCall>> {
        if self.tool_calls.is_empty() {
            None
        } else {
            Some(self.tool_calls.clone())
        }
    }
}

pub(crate) struct AfterCallModel<'a> {
    pub output: &'a ModelCallOutput,
}

pub(crate) struct BeforeCallTools<'a> {
    pub tool_calls: &'a [ToolCall],
}

pub(crate) struct AfterCallTools<'a> {
    pub tool_results: &'a [CallToolResult],
}

pub(crate) struct AfterStep<'a> {
    pub frame: &'a StepFrame,
    pub result: &'a StepResultDraft,
}

#[async_trait]
pub(crate) trait RuntimeHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn configure_execution_policy(&self, _state: &AgentState, _policy: &mut ExecutionPolicy) {}

    fn snapshot(&self) -> Option<Value> {
        None
    }

    async fn before_step(&self, _input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn before_call_model(&self) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn on_model_event(&self, _input: ModelEventCtx<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn after_call_model(&self, _input: AfterCallModel<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn before_call_tools(
        &self,
        _input: BeforeCallTools<'_>,
    ) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn after_call_tools(&self, _input: AfterCallTools<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }

    async fn after_step(&self, _input: AfterStep<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![])
    }
}
