use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde_json::json;

use crate::{
    prompts::upper_narrator_prompt,
    resources::{turn_state::TurnPhase, world_state::WorldState},
    utils::build_layer,
};

#[derive(Component)]
pub struct UpperNarrator;

impl UpperNarrator {
    pub fn new() -> Self {
        Self
    }

    pub fn build_task_context(&self, world_state: &WorldState, phase: TurnPhase) -> Context {
        let narration_goal = match phase {
            TurnPhase::NarratorScene => {
                "把当前场景编排结果写成开场叙事，为主角进入行动前的观察与选择建立氛围。"
            }
            TurnPhase::NarratorStory => {
                "把主角行动后的世界变化写成后续叙事，突出结果、反馈与局势推进。"
            }
            _ => "把当前世界状态写成连贯叙事。",
        };

        let prompt = upper_narrator_prompt::TASK_PROMPT
            .replace("{narration_goal}", narration_goal)
            .replace("{world_history}", &world_state.history_text())
            .replace("{latest_world_change}", &world_state.latest_history_entry_text())
            .replace("{current_location}", &world_state.current_location_text())
            .replace("{current_scene}", &world_state.current_scene_text())
            .replace("{npcs_state}", &world_state.npcs_state_text())
            .replace("{protagonist_state}", &world_state.protagonist_state_text())
            .replace("{item_locations}", &world_state.item_locations_text());

        Context::new().layer(build_layer(
            "upper-narrator-task",
            LayerKind::System,
            json!(prompt),
            100,
        ))
    }
}
