use async_trait::async_trait;

use super::{AfterStep, HookError, HookOutcome, RuntimeHook, StepHookContext, StepResultDraft};
use crate::agents::AgentActorEvent;

#[derive(Default)]
pub struct IterationEventHook;

#[async_trait]
impl RuntimeHook for IterationEventHook {
    fn name(&self) -> &'static str {
        "iteration_events"
    }

    async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        if !matches!(input.result, StepResultDraft::Error(_)) {
            ctx.send_event(AgentActorEvent::Iteration {
                iteration: ctx.state.iteration,
                message_count: ctx.state.context.conversation().len(),
            })
            .await;
        }
        Ok(HookOutcome::Continue)
    }
}
