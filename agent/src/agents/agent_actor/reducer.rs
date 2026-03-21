use crate::agents::agent_actor::lifecycle::{LifeCycleContext, LifeCycleEffect};

pub struct Reducer;

impl Reducer {
    pub async fn applay(ctx: LifeCycleContext, effect: LifeCycleEffect) -> LifeCycleContext {
        todo!("applay effect");
    }
}
