use agent::{
    agent::{Context, LayerKind},
    core::Message,
};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{prompts::upper_narrator_prompt::UPPER_NARRATOR_PROMPT, utils::build_layer};

#[derive(Component)]
pub struct UpperNarrator {
    context: Context,
}

impl UpperNarrator {
    pub fn new(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = UPPER_NARRATOR_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile);

        let context = Context::new().layer(build_layer(
            "upper-narrator-system-prompt",
            LayerKind::System,
            json!(system_prompt),
            100,
        ));
        Self { context }
    }

    pub fn from_context(context: Context) -> Self {
        Self { context }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn add_user_message(&mut self, content: &str) {
        self.context.add_message(Message::user(content));
    }

    pub fn append_assistant_message(&mut self, content: &str) {
        self.context.add_message(Message::assistant(content));
    }
}
