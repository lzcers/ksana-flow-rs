use crate::{prompts::fate_weaver_prompt, utils::build_layer};
use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;
// 标识 Entity
#[derive(Component)]
pub struct FateWeaver {
    context: Context,
}

impl FateWeaver {
    pub fn new(world_profile: &str, prota_profile: &str) -> Self {
        let system_prompt = fate_weaver_prompt::SYS_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", prota_profile);
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

// 记录推演的上下文
#[derive(Component, Default)]
pub struct FateLine {}
