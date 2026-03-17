use async_trait::async_trait;

use super::{AgentHook, HookError, HookOutcome, StepHookContext, StepResultDraft};
use crate::{agents::AgentActorEvent, agents::AgentError};

#[derive(Default)]
pub struct MaxIterationsHook;

#[async_trait]
impl AgentHook for MaxIterationsHook {
    fn name(&self) -> &'static str {
        "max_iterations"
    }

    async fn before_step(&self, ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError> {
        if ctx.state.iteration >= ctx.state.max_iterations {
            let iteration = ctx.state.iteration;
            ctx.send_event(AgentActorEvent::MaxIterations { iteration })
                .await;
            return Ok(HookOutcome::Finish(StepResultDraft::Error(
                AgentError::MaxIterations(iteration),
            )));
        }

        Ok(HookOutcome::Continue)
    }
}
