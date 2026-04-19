use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{fate_weaver::prompt::SYS_PROMPT, shared::build_layer};

#[derive(Component)]
pub struct FateWeaverContext {
    context: Context,
}
#[derive(Component)]
pub struct FateLine {
    pub max_rounds: u32,
    pub current_round: u32,
}

impl FateWeaverContext {
    pub fn new(prota_profile: String, world_profile: String) -> Self {
        let system_prompt = SYS_PROMPT
            .replace("{world_profile}", &world_profile)
            .replace("{protagonist_profile}", &prota_profile);
        let context = Context::new().layer(build_layer(
            "fate-weaver-system",
            LayerKind::System,
            json!(system_prompt),
            100,
        ));
        Self { context }
    }
    pub fn get_context(&self) -> &Context {
        &self.context
    }

    pub fn get_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }
}

impl FateLine {
    pub fn new(max_rounds: u32) -> Self {
        Self {
            max_rounds,
            current_round: 0,
        }
    }
}
