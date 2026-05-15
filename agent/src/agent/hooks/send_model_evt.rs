use crate::agent::{
    AgentActorEvent,
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow, LifeCycleResult},
    call_model::CallModelEvent,
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct SendModelEvtHook;

impl SendModelEvtHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for SendModelEvtHook {
    fn name(&self) -> HookName {
        "send_model_evt"
    }
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: &LifeCycle) -> bool {
        matches!(stage, LifeCycle::OnModelEvent)
    }
    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow {
        let event_tx = ctx.agent_tx.clone();
        let Some(tx) = event_tx else {
            return LifeCycleFlow::Continue(LifeCycleResult::None);
        };
        let evt = ctx.model_event.as_ref().unwrap();
        let mapped = match evt {
            CallModelEvent::TextChunk(content) => AgentActorEvent::ContentChunk(content.clone()),
            CallModelEvent::ReasoningChunk(content) => {
                AgentActorEvent::ReasoningChunk(content.clone())
            }
            CallModelEvent::Completed {
                content,
                reasoning_content,
                tools_call,
                ..
            } => AgentActorEvent::StepCompleted {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: tools_call.clone(),
            },
            CallModelEvent::Error(message) => AgentActorEvent::HookEvent {
                hook: "call_model".to_string(),
                kind: "error".to_string(),
                payload: serde_json::json!({ "message": message }),
            },
        };

        let _ = tx.send(mapped).await;

        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
