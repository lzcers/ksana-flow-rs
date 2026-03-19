use async_trait::async_trait;

use super::{AfterStep, HookError, HookOutcome, RuntimeHook, StepHookContext, StepResultDraft};
use crate::agents::AgentActorEvent;

#[derive(Default)]
pub struct ErrorEventHook;

#[async_trait]
impl RuntimeHook for ErrorEventHook {
    fn name(&self) -> &'static str {
        "error_events"
    }

    async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        if let StepResultDraft::Error(err) = input.result {
            ctx.send_event(AgentActorEvent::Error(err.clone())).await;
        }
        Ok(HookOutcome::Continue)
    }
}
