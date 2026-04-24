use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{prompts::protagonist_prompt, resources::world_state::WorldState, utils::build_layer};

#[derive(Component)]
pub struct Protagonist {
    protagonist_profile: String,
}

impl Protagonist {
    pub fn new(prota_profile: &str) -> Self {
        Self {
            protagonist_profile: prota_profile.to_string(),
        }
    }

    pub fn build_task_context(&self, world_state: &WorldState) -> Context {
        let prompt = protagonist_prompt::TASK_PROMPT
            .replace("{protagonist_profile}", &self.protagonist_profile)
            .replace("{world_history}", &world_state.history_text())
            .replace(
                "{latest_world_change}",
                &world_state.latest_history_entry_text(),
            )
            .replace("{current_location}", &world_state.current_location_text())
            .replace("{current_scene}", &world_state.current_scene_text())
            .replace("{npcs_state}", &world_state.npcs_state_text())
            .replace("{protagonist_state}", &world_state.protagonist_state_text())
            .replace("{item_locations}", &world_state.item_locations_text());

        Context::new().layer(build_layer(
            "protagonist-task",
            LayerKind::System,
            json!(prompt),
            100,
        ))
    }
}
