pub mod context_persistence;
pub mod error_events;
pub mod iteration_events;
pub mod lifecycle;
pub mod max_iterations;
pub mod metrics;
pub mod registry;
pub mod runtime;
pub mod streaming_events;
pub mod timeout_policy;

pub use context_persistence::ContextPersistenceHook;
pub use error_events::ErrorEventHook;
pub use iteration_events::IterationEventHook;
pub use lifecycle::LifecycleHook;
pub use max_iterations::MaxIterationsHook;
pub use metrics::{ExecutionMetrics, MetricsHook};
pub use registry::HookRegistry;
pub use runtime::{
    AfterCallModel, AfterCallTools, AfterStep, AgentHook, BeforeCallModel, BeforeCallTools,
    ExecutionPolicy, HookError, HookOutcome, HookPhase, ModelCallOutput, ModelEventCtx,
    StepHookContext, StepResultDraft, StepScratchpad,
};
pub use streaming_events::StreamingEventHook;
pub use timeout_policy::TimeoutPolicyHook;
