use std::collections::HashMap;

use serde_json::Value;

use super::{
    AfterCallModel, AfterCallTools, AfterStep, AgentHook, BeforeCallModel, BeforeCallTools,
    ContextPersistenceHook, ErrorEventHook, ExecutionPolicy, HookError, HookOutcome, HookPhase,
    IterationEventHook, LifecycleHook, MaxIterationsHook, MetricsHook, ModelEventCtx,
    StepHookContext, StepResultDraft, StreamingEventHook,
};
use crate::agents::{AgentError, AgentState};

pub struct HookRegistry {
    hooks: Vec<Box<dyn AgentHook>>,
}

impl HookRegistry {
    pub fn empty() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register<H>(mut self, hook: H) -> Self
    where
        H: AgentHook + 'static,
    {
        self.hooks.push(Box::new(hook));
        self
    }

    pub fn push<H>(&mut self, hook: H)
    where
        H: AgentHook + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(dyn AgentHook + '_)> {
        self.hooks.iter().map(|hook| hook.as_ref())
    }

    pub fn execution_policy(&self, state: &AgentState) -> ExecutionPolicy {
        let mut policy = ExecutionPolicy::default();
        for hook in self.iter() {
            hook.configure_execution_policy(state, &mut policy);
        }
        policy
    }

    pub fn snapshot(&self, name: &str) -> Option<Value> {
        self.iter()
            .find(|hook| hook.name() == name)
            .and_then(|hook| hook.snapshot())
    }

    pub fn snapshots(&self) -> HashMap<String, Value> {
        self.iter()
            .filter_map(|hook| {
                hook.snapshot()
                    .map(|snapshot| (hook.name().to_string(), snapshot))
            })
            .collect()
    }

    fn resolve_outcome(
        hook: &(dyn AgentHook + '_),
        phase: HookPhase,
        result: Result<HookOutcome, HookError>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        match result {
            Ok(HookOutcome::Continue) => Ok(None),
            Ok(HookOutcome::Finish(result)) => Ok(Some(result)),
            Err(err) => Err(AgentError::Hook {
                plugin: hook.name(),
                phase,
                message: err.message,
            }),
        }
    }

    pub async fn before_step(
        &self,
        ctx: &mut StepHookContext<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) =
                Self::resolve_outcome(hook, HookPhase::BeforeStep, hook.before_step(ctx).await)?
            {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn before_call_model(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut BeforeCallModel<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::BeforeCallModel,
                hook.before_call_model(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn on_model_event(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &ModelEventCtx<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::OnModelEvent,
                hook.on_model_event(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn after_call_model(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterCallModel<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::AfterCallModel,
                hook.after_call_model(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn before_call_tools(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut BeforeCallTools<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::BeforeCallTools,
                hook.before_call_tools(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn after_call_tools(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterCallTools<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::AfterCallTools,
                hook.after_call_tools(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterStep<'_>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.iter() {
            if let Some(result) = Self::resolve_outcome(
                hook,
                HookPhase::AfterStep,
                hook.after_step(ctx, input).await,
            )? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::empty()
            .register(MaxIterationsHook)
            .register(LifecycleHook)
            .register(MetricsHook::default())
            .register(StreamingEventHook)
            .register(ContextPersistenceHook)
            .register(IterationEventHook)
            .register(ErrorEventHook)
    }
}
