use async_trait::async_trait;

use super::{AfterStep, AgentHook, HookError, HookOutcome, StepHookContext, StepResultDraft};

#[derive(Default)]
pub struct LifecycleHook;

#[async_trait]
impl AgentHook for LifecycleHook {
    fn name(&self) -> &'static str {
        "lifecycle"
    }

    async fn before_step(&self, ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError> {
        ctx.state.iteration += 1;
        ctx.state.state = crate::agents::JobState::Running;
        Ok(HookOutcome::Continue)
    }

    async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        ctx.state.state = match input.result {
            StepResultDraft::Continue { .. } => crate::agents::JobState::WaitingInput,
            StepResultDraft::Done { .. } => crate::agents::JobState::Completed,
            StepResultDraft::Error(_) => crate::agents::JobState::Failed,
        };
        Ok(HookOutcome::Continue)
    }
}
