use crate::agents::agent_actor::lifecycle::{LifeCycleContext, LifeCycleResult};

pub struct Reducer;

impl Reducer {
    pub async fn applay(ctx: LifeCycleContext, result: LifeCycleResult) -> LifeCycleContext {
        todo!("applay effect");
    }
}
