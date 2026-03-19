use tokio::sync::mpsc;

use crate::agents::{AgentActorEvent, AgentError, AgentState, CallModelEvent, ToolDef};

use super::{
    AfterCallModel, AfterCallTools, AfterStep, AfterStepInput, BeforeCallModel, BeforeCallTools,
    BeforeStep, BeforeStepInput, Effect, EffectHandle, EffectSignal, HookEffect, HookError,
    HookPhase, HookRegistry, ModelEventCtx, ModelEventInput, RuntimeHookRegistry, StepFrame,
    StepResultDraft,
};

pub(crate) struct HookPipeline<'a> {
    runtime_hooks: &'a RuntimeHookRegistry,
    hooks: &'a HookRegistry,
}

impl<'a> HookPipeline<'a> {
    pub(crate) fn new(runtime_hooks: &'a RuntimeHookRegistry, hooks: &'a HookRegistry) -> Self {
        Self {
            runtime_hooks,
            hooks,
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

    fn into_public_effect(
        hook_name: &'static str,
        phase: HookPhase,
        effect: HookEffect,
    ) -> Result<Effect, AgentError> {
        match effect {
            HookEffect::EmitEvent(event) => Ok(Effect::EmitNow(AgentActorEvent::HookEvent {
                hook: hook_name.to_string(),
                kind: event.kind,
                payload: event.payload,
            })),
            HookEffect::ReplaceResult(next_result) => {
                if phase != HookPhase::AfterStep {
                    return Err(Self::hook_error(
                        hook_name,
                        phase,
                        "ReplaceResult is only supported during after_step",
                    ));
                }
                Ok(Effect::SetResult(next_result.into_draft()))
            }
            HookEffect::Abort { reason } => {
                Ok(Effect::Abort(Self::hook_error(hook_name, phase, reason)))
            }
            HookEffect::SetMetadata { key, value } => Ok(Effect::SetMetadata { key, value }),
            HookEffect::RemoveMetadata { key } => Ok(Effect::RemoveMetadata { key }),
        }
    }

    async fn apply_public_effects(
        hook_name: &'static str,
        phase: HookPhase,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        effects: Vec<HookEffect>,
    ) -> Result<EffectSignal, AgentError> {
        for effect in effects {
            let effect = Self::into_public_effect(hook_name, phase, effect)?;
            let signal = EffectHandle::apply(frame, event_tx, effect).await;
            if signal != EffectSignal::Continue {
                return Ok(signal);
            }
        }

        Ok(EffectSignal::Continue)
    }

    async fn apply_runtime_effects(
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        effects: Vec<Effect>,
    ) -> EffectSignal {
        for effect in effects {
            let signal = EffectHandle::apply(frame, event_tx, effect).await;
            if signal != EffectSignal::Continue {
                return signal;
            }
        }

        EffectSignal::Continue
    }

    pub(crate) async fn before_step(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<(), AgentError> {
        for hook in self.hooks.iter() {
            let input = BeforeStepInput::capture(state, &frame.metadata);
            let effects = hook
                .before_step(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::BeforeStep, message)
                })?;
            if Self::apply_public_effects(
                hook.name(),
                HookPhase::BeforeStep,
                frame,
                event_tx,
                effects,
            )
            .await?
                != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        for hook in self.runtime_hooks.iter() {
            let input = BeforeStep { state, frame };
            let effects = hook
                .before_step(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::BeforeStep, message)
                })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn before_call_model(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        tools: &[ToolDef],
    ) -> Result<(), AgentError> {
        for hook in self.runtime_hooks.iter() {
            let input = BeforeCallModel {
                state,
                frame,
                tools,
            };
            let effects =
                hook.before_call_model(input)
                    .await
                    .map_err(|HookError { message }| {
                        Self::hook_error(hook.name(), HookPhase::BeforeCallModel, message)
                    })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn on_model_event(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        event: &CallModelEvent,
    ) -> Result<(), AgentError> {
        for hook in self.hooks.iter() {
            let input = ModelEventInput::capture(state, event, &frame.metadata);
            let effects = hook
                .on_model_event(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::OnModelEvent, message)
                })?;
            if Self::apply_public_effects(
                hook.name(),
                HookPhase::OnModelEvent,
                frame,
                event_tx,
                effects,
            )
            .await?
                != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        for hook in self.runtime_hooks.iter() {
            let input = ModelEventCtx {
                state,
                frame,
                event,
            };
            let effects = hook
                .on_model_event(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::OnModelEvent, message)
                })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn after_call_model(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<(), AgentError> {
        for hook in self.runtime_hooks.iter() {
            let input = AfterCallModel {
                state,
                frame,
                output: &frame.model_output,
            };
            let effects = hook
                .after_call_model(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::AfterCallModel, message)
                })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn before_call_tools(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<(), AgentError> {
        for hook in self.runtime_hooks.iter() {
            let input = BeforeCallTools {
                state,
                frame,
                tool_calls: &frame.tool_calls,
            };
            let effects =
                hook.before_call_tools(input)
                    .await
                    .map_err(|HookError { message }| {
                        Self::hook_error(hook.name(), HookPhase::BeforeCallTools, message)
                    })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn after_call_tools(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> Result<(), AgentError> {
        for hook in self.runtime_hooks.iter() {
            let input = AfterCallTools {
                state,
                frame,
                tool_calls: &frame.tool_calls,
                tool_results: &frame.tool_results,
            };
            let effects = hook
                .after_call_tools(input)
                .await
                .map_err(|HookError { message }| {
                    Self::hook_error(hook.name(), HookPhase::AfterCallTools, message)
                })?;
            if Self::apply_runtime_effects(frame, event_tx, effects).await != EffectSignal::Continue
            {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) async fn after_step(
        &self,
        state: &AgentState,
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) {
        for hook in self.hooks.iter() {
            let Some(result) = frame.result() else {
                return;
            };
            let input = AfterStepInput::capture(state, result, &frame.metadata);
            let effects = match hook.after_step(input).await {
                Ok(effects) => effects,
                Err(HookError { message }) => {
                    frame.set_result(StepResultDraft::Error(Self::hook_error(
                        hook.name(),
                        HookPhase::AfterStep,
                        message,
                    )));
                    break;
                }
            };

            match Self::apply_public_effects(
                hook.name(),
                HookPhase::AfterStep,
                frame,
                event_tx,
                effects,
            )
            .await
            {
                Ok(EffectSignal::Continue | EffectSignal::ResultSet) => {}
                Ok(EffectSignal::Aborted) => break,
                Err(err) => {
                    frame.set_result(StepResultDraft::Error(err));
                    break;
                }
            }
        }

        for hook in self.runtime_hooks.iter() {
            let Some(result) = frame.result() else {
                return;
            };
            let input = AfterStep {
                state,
                frame,
                result,
            };
            let effects = match hook.after_step(input).await {
                Ok(effects) => effects,
                Err(HookError { message }) => {
                    frame.set_result(StepResultDraft::Error(Self::hook_error(
                        hook.name(),
                        HookPhase::AfterStep,
                        message,
                    )));
                    continue;
                }
            };

            let _ = Self::apply_runtime_effects(frame, event_tx, effects).await;
        }
    }
}
