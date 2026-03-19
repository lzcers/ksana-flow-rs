pub mod agent_actor;
// pub mod agent_loop;
pub mod agent_state;
pub mod call_model;
pub mod context;
pub mod hooks;
#[cfg(test)]
mod tests;
pub mod tools;

pub use agent_actor::{
    AgentActor, AgentActorBuilder, AgentActorCommand, AgentActorEvent, AgentActorHandle,
    AgentError, StepResult,
};
pub use agent_state::{AgentState, JobState};
pub use call_model::{CallModelEvent, CallToolResult, call_model, call_tool, call_tools};
pub use context::{Context, Layer, LayerKind, LayerMeta};
pub use hooks::{
    AfterStepInput, BeforeStepInput, Hook, HookContinueStep, HookDoneStep, HookEffect, HookError,
    HookEvent, HookModelEvent, HookPhase, HookRegistry, HookStepError, HookStepResult,
    HookStepUpdate, HookToolCall, HookToolCallFunction, HookToolResult, ModelEventInput,
};

pub(crate) use hooks::{
    AfterCallModel, AfterCallTools, AfterStep, BeforeCallModel, BeforeCallTools,
    ContextPersistenceHook, ExecutionMetrics, HookOutcome, IterationEventHook, LifecycleHook,
    ModelEventCtx, RuntimeHook, RuntimeHookRegistry, StepHookContext, StepResultDraft,
    TimeoutPolicyHook,
};

pub use tools::{
    GenericToolExecutor, Tool, ToolCall, ToolCallFunction, ToolDef, ToolExecutor,
    ToolExecutorError, ToolRegistry, ToolResult,
};
