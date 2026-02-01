use tokio::sync::mpsc;

use super::{CTX_FLOW_EVENT_SENDER, Context, FlowEvent};

#[derive(Clone, Default)]
pub struct RuntimeServices {
    pub event_sender: Option<mpsc::Sender<FlowEvent>>,
}

impl RuntimeServices {
    pub fn from_context(ctx: &Context) -> Self {
        let event_sender = ctx
            .get_any::<mpsc::Sender<FlowEvent>>(CTX_FLOW_EVENT_SENDER)
            .map(|s| (*s).clone());
        Self { event_sender }
    }

    // 把 RuntimeServices 中的 event_sender attach 到 Context 中
    pub fn attach_to_context(&self, ctx: &Context) {
        if let Some(sender) = self.event_sender.as_ref() {
            ctx.set_any(CTX_FLOW_EVENT_SENDER, sender.clone());
        }
    }
}
