use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{HookError, StepResultDraft};
use crate::agents::{AgentError, AgentState, CallModelEvent, CallToolResult, JobState, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookToolCall {
    pub id: String,
    pub call_type: String,
    pub index: Option<u32>,
    pub function: HookToolCallFunction,
}

impl HookToolCall {
    pub(crate) fn from_tool_call(call: &ToolCall) -> Self {
        Self {
            id: call.id.clone(),
            call_type: call
                .call_type
                .clone()
                .unwrap_or_else(|| "function".to_string()),
            index: call.index,
            function: if let Some(function) = call.function.as_ref() {
                HookToolCallFunction {
                    name: function.name.clone(),
                    arguments: function.arguments.clone(),
                }
            } else {
                HookToolCallFunction {
                    name: call.get_name(),
                    arguments: serde_json::to_string(&call.get_arguments())
                        .unwrap_or_else(|_| "null".to_string()),
                }
            },
        }
    }

    pub(crate) fn into_tool_call(self) -> ToolCall {
        ToolCall {
            id: self.id,
            call_type: Some(self.call_type),
            index: self.index,
            function: Some(crate::agents::ToolCallFunction {
                name: self.function.name,
                arguments: self.function.arguments,
            }),
            name: None,
            arguments: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    pub output: String,
}

impl HookToolResult {
    pub(crate) fn from_call_tool_result(result: &CallToolResult) -> Self {
        Self {
            call_id: result.call_id.clone(),
            tool_name: result.tool_name.clone(),
            success: result.success,
            output: result.output.clone(),
        }
    }

    pub(crate) fn into_call_tool_result(self) -> CallToolResult {
        CallToolResult {
            call_id: self.call_id,
            tool_name: self.tool_name,
            success: self.success,
            output: self.output,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookStepError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookContinueStep {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<HookToolCall>,
    pub tool_results: Vec<HookToolResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookDoneStep {
    pub content: String,
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookStepResult {
    Continue(HookContinueStep),
    Done(HookDoneStep),
    Error(HookStepError),
}

impl HookStepResult {
    pub(crate) fn from_draft(draft: &StepResultDraft) -> Self {
        match draft {
            StepResultDraft::Continue {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            } => Self::Continue(HookContinueStep {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: tool_calls
                    .iter()
                    .map(HookToolCall::from_tool_call)
                    .collect(),
                tool_results: tool_results
                    .iter()
                    .map(HookToolResult::from_call_tool_result)
                    .collect(),
            }),
            StepResultDraft::Done {
                content,
                reasoning_content,
            } => Self::Done(HookDoneStep {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
            }),
            StepResultDraft::Error(err) => Self::Error(HookStepError {
                kind: hook_error_kind(err).to_string(),
                message: err.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookStepUpdate {
    Continue(HookContinueStep),
    Done(HookDoneStep),
}

impl HookStepUpdate {
    pub(crate) fn into_draft(self) -> StepResultDraft {
        match self {
            Self::Continue(HookContinueStep {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            }) => StepResultDraft::Continue {
                content,
                reasoning_content,
                tool_calls: tool_calls
                    .into_iter()
                    .map(HookToolCall::into_tool_call)
                    .collect(),
                tool_results: tool_results
                    .into_iter()
                    .map(HookToolResult::into_call_tool_result)
                    .collect(),
            },
            Self::Done(HookDoneStep {
                content,
                reasoning_content,
            }) => StepResultDraft::Done {
                content,
                reasoning_content,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookModelEvent {
    TextChunk(String),
    ReasoningChunk(String),
    Completed {
        content: String,
        reasoning_content: Option<String>,
        tool_calls: Vec<HookToolCall>,
    },
    Error {
        message: String,
    },
}

impl HookModelEvent {
    pub(crate) fn from_event(event: &CallModelEvent) -> Self {
        match event {
            CallModelEvent::TextChunk(text) => Self::TextChunk(text.clone()),
            CallModelEvent::ReasoningChunk(text) => Self::ReasoningChunk(text.clone()),
            CallModelEvent::Completed {
                content,
                reasoning_content,
                tool_calls,
            } => Self::Completed {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: tool_calls
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .map(HookToolCall::from_tool_call)
                    .collect(),
            },
            CallModelEvent::Error(message) => Self::Error {
                message: message.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeforeStepInput {
    pub job_id: Uuid,
    pub user_id: String,
    pub conversation_id: Option<String>,
    pub iteration: usize,
    pub max_iterations: usize,
    pub job_state: JobState,
    pub context_len: usize,
    pub metadata: HashMap<String, Value>,
}

impl BeforeStepInput {
    pub(crate) fn capture(state: &AgentState, metadata: &HashMap<String, Value>) -> Self {
        Self {
            job_id: state.job_id,
            user_id: state.user_id.clone(),
            conversation_id: state.conversation_id.map(|id| id.to_string()),
            iteration: state.iteration,
            max_iterations: state.max_iterations,
            job_state: state.state,
            context_len: state.context.conversation().len(),
            metadata: metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AfterStepInput {
    pub job_id: Uuid,
    pub iteration: usize,
    pub job_state: JobState,
    pub result: HookStepResult,
    pub metadata: HashMap<String, Value>,
}

impl AfterStepInput {
    pub(crate) fn capture(
        state: &AgentState,
        result: &StepResultDraft,
        metadata: &HashMap<String, Value>,
    ) -> Self {
        Self {
            job_id: state.job_id,
            iteration: state.iteration,
            job_state: state.state,
            result: HookStepResult::from_draft(result),
            metadata: metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelEventInput {
    pub job_id: Uuid,
    pub iteration: usize,
    pub event: HookModelEvent,
    pub metadata: HashMap<String, Value>,
}

impl ModelEventInput {
    pub(crate) fn capture(
        state: &AgentState,
        event: &CallModelEvent,
        metadata: &HashMap<String, Value>,
    ) -> Self {
        Self {
            job_id: state.job_id,
            iteration: state.iteration,
            event: HookModelEvent::from_event(event),
            metadata: metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookEvent {
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookEffect {
    EmitEvent(HookEvent),
    ReplaceResult(HookStepUpdate),
    Abort { reason: String },
    SetMetadata { key: String, value: Value },
    RemoveMetadata { key: String },
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &'static str;

    async fn before_step(&self, _input: BeforeStepInput) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![])
    }

    async fn on_model_event(&self, _input: ModelEventInput) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![])
    }

    async fn after_step(&self, _input: AfterStepInput) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![])
    }
}

pub struct HookRegistry {
    hooks: Vec<Box<dyn Hook>>,
}

impl HookRegistry {
    pub fn empty() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register<H>(mut self, hook: H) -> Self
    where
        H: Hook + 'static,
    {
        self.hooks.push(Box::new(hook));
        self
    }

    pub fn push<H>(&mut self, hook: H)
    where
        H: Hook + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(dyn Hook + '_)> {
        self.hooks.iter().map(|hook| hook.as_ref())
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

fn hook_error_kind(err: &AgentError) -> &'static str {
    match err {
        AgentError::Model(_) => "model",
        AgentError::Tool(_) => "tool",
        AgentError::Hook { .. } => "hook",
        AgentError::Cancelled => "cancelled",
        AgentError::Timeout => "timeout",
        AgentError::MaxIterations(_) => "max_iterations",
    }
}
