use async_trait::async_trait;

use super::{
    AfterCallModel, AfterCallTools, BeforeCallTools, Effect, HookError, ModelEventCtx, RuntimeHook,
};
use crate::agents::{AgentActorEvent, CallModelEvent};

#[derive(Default)]
pub struct StreamingEventHook;

#[async_trait]
impl RuntimeHook for StreamingEventHook {
    fn name(&self) -> &'static str {
        "streaming_events"
    }

    async fn on_model_event(&self, input: ModelEventCtx<'_>) -> Result<Vec<Effect>, HookError> {
        let effect = match input.event {
            CallModelEvent::TextChunk(text) => {
                Some(Effect::EmitNow(AgentActorEvent::ContentChunk(text.clone())))
            }
            CallModelEvent::ReasoningChunk(text) => Some(Effect::EmitNow(
                AgentActorEvent::ReasoningChunk(text.clone()),
            )),
            CallModelEvent::Completed { .. } | CallModelEvent::Error(_) => None,
        };
        Ok(effect.into_iter().collect())
    }

    async fn after_call_model(&self, input: AfterCallModel<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(vec![Effect::EmitNow(AgentActorEvent::StepCompleted {
            content: input.output.content.clone(),
            reasoning_content: input.output.reasoning_content.clone(),
            tool_calls: input.output.tool_calls_option(),
        })])
    }

    async fn before_call_tools(
        &self,
        input: BeforeCallTools<'_>,
    ) -> Result<Vec<Effect>, HookError> {
        if input.tool_calls.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![Effect::EmitNow(AgentActorEvent::ToolCalls(
                input.tool_calls.to_vec(),
            ))])
        }
    }

    async fn after_call_tools(&self, input: AfterCallTools<'_>) -> Result<Vec<Effect>, HookError> {
        Ok(input
            .tool_results
            .iter()
            .map(|result| {
                Effect::EmitNow(AgentActorEvent::ToolResult {
                    call_id: result.call_id.clone(),
                    success: result.success,
                    output: result.output.clone(),
                })
            })
            .collect())
    }
}
