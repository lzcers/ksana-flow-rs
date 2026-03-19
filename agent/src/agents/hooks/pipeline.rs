use std::collections::HashMap;

use serde_json::Value;
use tokio::sync::mpsc;

use crate::agents::{
    AgentActorEvent, AgentError, AgentState, CallModelEvent, CallToolResult, ToolCall, ToolDef,
};

use super::{
    AfterCallModel, AfterCallTools, AfterStep, AfterStepInput, BeforeCallModel, BeforeCallTools,
    BeforeStepInput, HookEffect, HookError, HookPhase, HookRegistry, ModelCallOutput,
    ModelEventCtx, ModelEventInput, RuntimeHookRegistry, StepHookContext, StepResultDraft,
    StepScratchpad,
};

pub(crate) struct HookPipeline<'a> {
    runtime_hooks: &'a RuntimeHookRegistry,
    hooks: &'a HookRegistry,
    scratchpad: StepScratchpad,
    metadata: HashMap<String, Value>,
}

impl<'a> HookPipeline<'a> {
    pub(crate) fn new(runtime_hooks: &'a RuntimeHookRegistry, hooks: &'a HookRegistry) -> Self {
        Self {
            runtime_hooks,
            hooks,
            scratchpad: StepScratchpad::default(),
            metadata: HashMap::new(),
        }
    }

    fn make_ctx<'b>(
        state: &'b mut AgentState,
        event_tx: Option<&'b mpsc::Sender<AgentActorEvent>>,
        scratchpad: &'b mut StepScratchpad,
    ) -> StepHookContext<'b> {
        StepHookContext {
            state,
            event_tx,
            scratchpad,
        }
    }

    fn hook_error(
        hook_name: &'static str,
        phase: HookPhase,
        message: impl Into<String>,
    ) -> AgentError {
        AgentError::Hook {
            plugin: hook_name,
            phase,
            message: message.into(),
        }
    }

    async fn apply_effects(
        &mut self,
        hook_name: &'static str,
        phase: HookPhase,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        effects: Vec<HookEffect>,
        mut current_result: Option<&mut StepResultDraft>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for effect in effects {
            match effect {
                HookEffect::EmitEvent(event) => {
                    if let Some(tx) = event_tx {
                        let _ = tx
                            .send(AgentActorEvent::HookEvent {
                                hook: hook_name.to_string(),
                                kind: event.kind,
                                payload: event.payload,
                            })
                            .await;
                    }
                }
                HookEffect::ReplaceResult(next_result) => {
                    if phase != HookPhase::AfterStep {
                        return Err(Self::hook_error(
                            hook_name,
                            phase,
                            "ReplaceResult is only supported during after_step",
                        ));
                    }

                    let Some(result) = current_result.as_deref_mut() else {
                        return Err(Self::hook_error(
                            hook_name,
                            phase,
                            "missing step result while applying ReplaceResult",
                        ));
                    };

                    *result = next_result.into_draft();
                }
                HookEffect::Abort { reason } => {
                    let next_result =
                        StepResultDraft::Error(Self::hook_error(hook_name, phase, reason));
                    if let Some(result) = current_result.as_deref_mut() {
                        *result = next_result.clone();
                    }
                    return Ok(Some(next_result));
                }
                HookEffect::SetMetadata { key, value } => {
                    self.metadata.insert(key, value);
                }
                HookEffect::RemoveMetadata { key } => {
                    self.metadata.remove(&key);
                }
            }
        }

        Ok(None)
    }

    async fn resolve_hook(
        &mut self,
        hook_name: &'static str,
        phase: HookPhase,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        result: Result<Vec<HookEffect>, HookError>,
        current_result: Option<&mut StepResultDraft>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        let effects = match result {
            Ok(effects) => effects,
            Err(HookError { message }) => {
                return Ok(Some(StepResultDraft::Error(Self::hook_error(
                    hook_name, phase, message,
                ))));
            }
        };

        self.apply_effects(hook_name, phase, event_tx, effects, current_result)
            .await
    }

    async fn run_hooks_before_step(
        &mut self,
        state: &AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.hooks.iter() {
            let input = BeforeStepInput::capture(state, &self.metadata);
            if let Some(result) = self
                .resolve_hook(
                    hook.name(),
                    HookPhase::BeforeStep,
                    event_tx,
                    hook.before_step(input).await,
                    None,
                )
                .await?
            {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    async fn run_hooks_on_model_event(
        &mut self,
        state: &AgentState,
        event: &CallModelEvent,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.hooks.iter() {
            let input = ModelEventInput::capture(state, event, &self.metadata);
            if let Some(result) = self
                .resolve_hook(
                    hook.name(),
                    HookPhase::OnModelEvent,
                    event_tx,
                    hook.on_model_event(input).await,
                    None,
                )
                .await?
            {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    async fn run_hooks_after_step(
        &mut self,
        state: &AgentState,
        result: &mut StepResultDraft,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        for hook in self.hooks.iter() {
            let input = AfterStepInput::capture(state, result, &self.metadata);
            if let Some(next_result) = self
                .resolve_hook(
                    hook.name(),
                    HookPhase::AfterStep,
                    event_tx,
                    hook.after_step(input).await,
                    Some(result),
                )
                .await?
            {
                return Ok(Some(next_result));
            }
        }

        Ok(None)
    }

    pub(crate) async fn before_step(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        if let Some(result) = self.run_hooks_before_step(&*state, event_tx).await? {
            return Ok(Some(result));
        }

        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        self.runtime_hooks.before_step(&mut ctx).await
    }

    pub(crate) async fn before_call_model(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        tools: &[ToolDef],
    ) -> Result<Option<StepResultDraft>, AgentError> {
        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let mut input = BeforeCallModel { tools };
        self.runtime_hooks
            .before_call_model(&mut ctx, &mut input)
            .await
    }

    pub(crate) async fn on_model_event(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        event: &CallModelEvent,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        if let Some(result) = self
            .run_hooks_on_model_event(&*state, event, event_tx)
            .await?
        {
            return Ok(Some(result));
        }

        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let input = ModelEventCtx { event };
        self.runtime_hooks.on_model_event(&mut ctx, &input).await
    }

    pub(crate) async fn after_call_model(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        output: &mut ModelCallOutput,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let mut input = AfterCallModel { output };
        self.runtime_hooks
            .after_call_model(&mut ctx, &mut input)
            .await
    }

    pub(crate) async fn before_call_tools(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        tool_calls: &mut Vec<ToolCall>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let mut input = BeforeCallTools { tool_calls };
        self.runtime_hooks
            .before_call_tools(&mut ctx, &mut input)
            .await
    }

    pub(crate) async fn after_call_tools(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        tool_calls: &[ToolCall],
        tool_results: &mut Vec<CallToolResult>,
    ) -> Result<Option<StepResultDraft>, AgentError> {
        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let mut input = AfterCallTools {
            tool_calls,
            tool_results,
        };
        self.runtime_hooks
            .after_call_tools(&mut ctx, &mut input)
            .await
    }

    pub(crate) async fn after_step(
        &mut self,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        result: &mut StepResultDraft,
    ) -> Result<(), AgentError> {
        if let Some(next_result) = self.run_hooks_after_step(&*state, result, event_tx).await? {
            return match next_result {
                StepResultDraft::Error(err) => Err(err),
                other => {
                    *result = other;
                    Ok(())
                }
            };
        }

        let mut ctx = Self::make_ctx(state, event_tx, &mut self.scratchpad);
        let mut input = AfterStep { result };
        if let Some(next_result) = self.runtime_hooks.after_step(&mut ctx, &mut input).await? {
            *input.result = next_result;
        }
        Ok(())
    }
}
