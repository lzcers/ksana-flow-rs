use agent::agent::{Context, LayerKind};
use agent::core::Message;
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{
    prompts::fate_weaver_prompt::{FATE_BASE_SYSTEM_PROMPT, OUTPUT_SCHEMA},
    utils::build_layer,
};
// 标识 Entity
#[derive(Component)]
pub struct FateWeaver {
    context: Context,
}

impl FateWeaver {
    pub fn new(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = FATE_BASE_SYSTEM_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile)
            .replace("{output_schema}", OUTPUT_SCHEMA);

        let context = Context::new()
            .layer(build_layer(
                "fate-system-prompt",
                LayerKind::System,
                json!(system_prompt),
                100,
            ))
            .layer(build_layer(
                "fate-conversation",
                LayerKind::Conversation,
                json!([]),
                0,
            ));

        Self { context }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn revert(&mut self) {
        self.context.rollback_latest_input();
    }
    pub fn append_user_message(&mut self, content: &str) {
        self.context.add_message(Message::user(content));
    }

    pub fn append_assistant_message(&mut self, content: &str) {
        self.context.add_message(Message::assistant(content));
    }
}
