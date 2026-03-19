use std::{any::Any, collections::HashMap, time::Duration};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::agents::{
    AgentActorEvent, AgentError, AgentState, CallModelEvent, CallToolResult, ToolCall, ToolDef,
};

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

#[derive(Debug, Clone)]
pub(crate) enum HookOutcome {
    Continue,
    Finish(StepResultDraft),
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
    pub(crate) fn insert<T: Send + Sync + 'static>(&mut self, key: &'static str, value: T) {
        self.inner.insert(key, Box::new(value));
    }

    pub(crate) fn get<T: Send + Sync + 'static>(&self, key: &'static str) -> Option<&T> {
        self.inner.get(key)?.downcast_ref::<T>()
    }

    pub(crate) fn get_mut<T: Send + Sync + 'static>(
        &mut self,
        key: &'static str,
    ) -> Option<&mut T> {
        self.inner.get_mut(key)?.downcast_mut::<T>()
    }
}

pub(crate) struct StepHookContext<'a> {
    pub state: &'a mut AgentState,
    pub event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    pub scratchpad: &'a mut StepScratchpad,
}

impl StepHookContext<'_> {
    pub(crate) async fn send_event(&self, event: AgentActorEvent) {
        if let Some(tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

#[allow(dead_code)]
pub(crate) struct BeforeCallModel<'a> {
    pub tools: &'a [ToolDef],
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
    pub output: &'a mut ModelCallOutput,
}

pub(crate) struct BeforeCallTools<'a> {
    pub tool_calls: &'a mut Vec<ToolCall>,
}

pub(crate) struct AfterCallTools<'a> {
    pub tool_calls: &'a [ToolCall],
    pub tool_results: &'a mut Vec<CallToolResult>,
}

pub(crate) struct AfterStep<'a> {
    pub result: &'a mut StepResultDraft,
}

#[async_trait]
pub(crate) trait RuntimeHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn configure_execution_policy(&self, _state: &AgentState, _policy: &mut ExecutionPolicy) {}

    fn snapshot(&self) -> Option<Value> {
        None
    }

    async fn before_step(&self, _ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn before_call_model(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &mut BeforeCallModel<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn on_model_event(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &ModelEventCtx<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn after_call_model(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &mut AfterCallModel<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn before_call_tools(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &mut BeforeCallTools<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn after_call_tools(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &mut AfterCallTools<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }

    async fn after_step(
        &self,
        _ctx: &mut StepHookContext<'_>,
        _input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        Ok(HookOutcome::Continue)
    }
}
