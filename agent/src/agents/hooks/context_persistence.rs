use async_trait::async_trait;

use super::{AfterStep, AgentHook, HookError, HookOutcome, StepHookContext, StepResultDraft};
use crate::core::Message;

#[derive(Default)]
pub struct ContextPersistenceHook;

#[async_trait]
impl AgentHook for ContextPersistenceHook {
    fn name(&self) -> &'static str {
        "context_persistence"
    }

    async fn after_step(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterStep<'_>,
    ) -> Result<HookOutcome, HookError> {
        match input.result {
            StepResultDraft::Continue {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            } => {
                ctx.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                });
                for result in tool_results.iter() {
                    ctx.state.context.add_message(Message::Tool {
                        tool_call_id: result.call_id.clone(),
                        content: result.output.clone(),
                    });
                }
            }
            StepResultDraft::Done {
                content,
                reasoning_content,
            } => {
                ctx.state.context.add_message(Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: None,
                });
            }
            StepResultDraft::Error(_) => {}
        }
        Ok(HookOutcome::Continue)
    }
}
