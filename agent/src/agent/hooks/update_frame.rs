use crate::agent::{
    agent_actor::lifecycle::{LifeCycle, LifeCycleFlow, LifeCycleResult, ModelOuput},
    call_model::CallModelEvent,
    hooks::{HookName, LifeCycleContext, LifeCycleHook},
};

pub struct UpdateFrameHook {}

impl UpdateFrameHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl LifeCycleHook for UpdateFrameHook {
    fn name(&self) -> HookName {
        "update_frame"
    }
    fn priority(&self) -> i32 {
        0
    }
    fn on(&self, stage: &LifeCycle) -> bool {
        matches!(stage, LifeCycle::OnModelEvent)
    }
    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow {
        if let Some(CallModelEvent::Completed {
            content,
            reasoning_content,
            tools_call,
            usage,
        }) = ctx.model_event.as_ref()
        {
            let output = ModelOuput {
                content: content.to_owned(),
                reasoning_content: reasoning_content.to_owned(),
                tools_call: tools_call.to_owned(),
            };
            if let Some(usage) = usage {
                ctx.set_frame_token_usage(usage.clone());
            }
            ctx.set_frame_model_output(output);
        }

        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
}
