pub mod agent_actor;
// pub mod agent_loop;
pub mod agent_state;
pub mod agent_utils;
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
pub use agent_utils::{CallModelEvent, CallToolResult, call_model, call_tool, call_tools};
pub use context::{Context, Layer, LayerKind, LayerMeta};
pub use hooks::{
    AfterCallModel, AfterCallTools, AfterStep, AgentHook, BeforeCallModel, BeforeCallTools,
    ContextPersistenceHook, ErrorEventHook, ExecutionMetrics, ExecutionPolicy, HookError,
    HookOutcome, HookPhase, HookRegistry, IterationEventHook, LifecycleHook, MaxIterationsHook,
    MetricsHook, ModelCallOutput, ModelEventCtx, StepHookContext, StepResultDraft, StepScratchpad,
    StreamingEventHook, TimeoutPolicyHook,
};

pub use tools::{
    GenericToolExecutor, Tool, ToolCall, ToolCallFunction, ToolDef, ToolExecutor,
    ToolExecutorError, ToolRegistry, ToolResult,
};
