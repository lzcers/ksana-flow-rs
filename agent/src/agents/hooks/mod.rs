mod context_persistence;
mod error_events;
mod iteration_events;
mod lifecycle;
mod max_iterations;
mod metrics;
mod pipeline;
mod public;
mod registry;
mod runtime;
mod streaming_events;
mod timeout_policy;

pub use public::{
    AfterStepInput, BeforeStepInput, Hook, HookContinueStep, HookDoneStep, HookEffect, HookEvent,
    HookModelEvent, HookRegistry, HookStepError, HookStepResult, HookStepUpdate, HookToolCall,
    HookToolCallFunction, HookToolResult, ModelEventInput,
};
pub use runtime::{HookError, HookPhase};

pub(crate) use context_persistence::ContextPersistenceHook;
pub(crate) use error_events::ErrorEventHook;
pub(crate) use iteration_events::IterationEventHook;
pub(crate) use lifecycle::LifecycleHook;
pub(crate) use max_iterations::MaxIterationsHook;
pub(crate) use metrics::{ExecutionMetrics, MetricsHook};
pub(crate) use pipeline::HookPipeline;
pub(crate) use registry::RuntimeHookRegistry;
pub(crate) use runtime::{
    AfterCallModel, AfterCallTools, AfterStep, BeforeCallModel, BeforeCallTools, ExecutionPolicy,
    HookOutcome, ModelCallOutput, ModelEventCtx, RuntimeHook, StepHookContext, StepResultDraft,
    StepScratchpad,
};
pub(crate) use streaming_events::StreamingEventHook;
pub(crate) use timeout_policy::TimeoutPolicyHook;
