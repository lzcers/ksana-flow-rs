use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{prompts::upper_narrator_prompt, utils::build_layer};

#[derive(Component)]
pub struct UpperNarrator {
    context: Context,
}

impl UpperNarrator {
    pub fn new() -> Self {
        Self {
            context: Context::new().layer(build_layer(
                "upper-narrator-system",
                LayerKind::System,
                json!(upper_narrator_prompt::SYS_PROMPT),
                100,
            )),
        }
    }

    pub fn get_context(&self) -> &Context {
        &self.context
    }

    pub fn get_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }
}

#[derive(Component)]
struct UpperNaraatorContext {}
