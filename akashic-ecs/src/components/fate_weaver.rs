use std::collections::HashMap;

use crate::{prompts::fate_weaver_prompt, utils::build_layer};
use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};
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
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateLine {
    pub latest: Option<FateFrame>,
    pub history: Vec<FateFrame>,
}

impl FateLine {
    pub fn push_frame(&mut self, frame: FateFrame) {
        self.latest = Some(frame.clone());
        self.history.push(frame);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateFrame {
    pub chapter: String,
    pub section: String,
    pub time: String,
    pub location: String,
    pub environment: String,
    #[serde(default)]
    pub characters: Vec<FateCharacterState>,
    pub event: String,
    pub cause: String,
    pub situation: String,
    #[serde(default)]
    pub info_gained: Vec<String>,
    #[serde(default)]
    pub foreshadowing: Vec<ForeshadowingChange>,
    #[serde(default)]
    pub ending: FateEnding,
    #[serde(default)]
    pub pacing: FatePacing,
    #[serde(default)]
    pub choices: Vec<FateChoice>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateCharacterState {
    pub name: String,
    pub observable: String,
    #[serde(default)]
    pub deltas: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForeshadowingChange {
    pub id: String,
    pub op: String,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateEnding {
    #[serde(default)]
    pub missing_items: Vec<String>,
    #[serde(default)]
    pub completed_milestones: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FatePacing {
    pub beat: String,
    pub tension: String,
    pub next_hint: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateChoice {
    pub id: String,
    pub text: String,
    pub next_trigger: String,
}
