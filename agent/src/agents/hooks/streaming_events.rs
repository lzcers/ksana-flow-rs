use async_trait::async_trait;

use super::{
    AfterCallModel, AfterCallTools, AgentHook, BeforeCallTools, HookError, HookOutcome,
    ModelEventCtx, StepHookContext,
};
use crate::agents::{AgentActorEvent, CallModelEvent};

#[derive(Default)]
pub struct StreamingEventHook;

#[async_trait]
impl AgentHook for StreamingEventHook {
    fn name(&self) -> &'static str {
        "streaming_events"
    }

    async fn on_model_event(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &ModelEventCtx<'_>,
    ) -> Result<HookOutcome, HookError> {
        match input.event {
            CallModelEvent::TextChunk(text) => {
                ctx.send_event(AgentActorEvent::ContentChunk(text.clone()))
                    .await;
            }
            CallModelEvent::ReasoningChunk(text) => {
                ctx.send_event(AgentActorEvent::ReasoningChunk(text.clone()))
                    .await;
            }
            CallModelEvent::Completed { .. } | CallModelEvent::Error(_) => {}
        }
        Ok(HookOutcome::Continue)
    }

    async fn after_call_model(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterCallModel<'_>,
    ) -> Result<HookOutcome, HookError> {
        ctx.send_event(AgentActorEvent::StepCompleted {
            content: input.output.content.clone(),
            reasoning_content: input.output.reasoning_content.clone(),
            tool_calls: input.output.tool_calls_option(),
        })
        .await;
        Ok(HookOutcome::Continue)
    }

    async fn before_call_tools(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut BeforeCallTools<'_>,
    ) -> Result<HookOutcome, HookError> {
        if !input.tool_calls.is_empty() {
            ctx.send_event(AgentActorEvent::ToolCalls(input.tool_calls.clone()))
                .await;
        }
        Ok(HookOutcome::Continue)
    }

    async fn after_call_tools(
        &self,
        ctx: &mut StepHookContext<'_>,
        input: &mut AfterCallTools<'_>,
    ) -> Result<HookOutcome, HookError> {
        for result in input.tool_results.iter() {
            ctx.send_event(AgentActorEvent::ToolResult {
                call_id: result.call_id.clone(),
                success: result.success,
                output: result.output.clone(),
            })
            .await;
        }
        Ok(HookOutcome::Continue)
    }
}
