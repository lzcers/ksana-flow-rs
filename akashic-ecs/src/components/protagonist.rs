use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{prompts::protagonist_prompt, utils::build_layer};

#[derive(Component)]
pub struct Protagonist {
    context: Context,
}
impl Protagonist {
    pub fn new(prota_profile: &str) -> Self {
        Self {
            context: Context::new().layer(build_layer(
                "protagonist-system",
                LayerKind::System,
                json!(
                    protagonist_prompt::SYS_PROMPT.replace("{protagonist_profile}", prota_profile)
                ),
                100,
            )),
        }
    }

    pub fn get_context(&self) -> &Context {
        &self.context
    }
}

#[derive(Component)]
struct ProtagonistContext {}
