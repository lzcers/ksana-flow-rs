use bevy_ecs::resource::Resource;

use crate::{
    components::fate_weaver::{FateCharacterState, FateFrame},
    resources::turn_state::TurnPhase,
};

#[derive(Resource, Debug, Clone, Default)]
pub struct WorldState {
    pub history: Vec<String>,
    pub current_location: String,
    pub current_scene: String,
    pub npcs_state: Vec<String>,
    pub protagonist_state: String,
    pub item_locations: Vec<String>,
}

impl WorldState {
    pub fn history_text(&self) -> String {
        Self::format_list(&self.history, "暂无历史事件记录")
    }

    pub fn npcs_state_text(&self) -> String {
        Self::format_list(&self.npcs_state, "暂无 NPC 状态")
    }

    pub fn protagonist_state_text(&self) -> String {
        Self::format_scalar(&self.protagonist_state, "暂无主角状态")
    }

    pub fn item_locations_text(&self) -> String {
        Self::format_list(&self.item_locations, "暂无物品位置信息")
    }

    pub fn current_location_text(&self) -> String {
        Self::format_scalar(&self.current_location, "未知位置")
    }

    pub fn current_scene_text(&self) -> String {
        Self::format_scalar(&self.current_scene, "暂无场景事实")
    }

    pub fn latest_history_entry_text(&self) -> String {
        self.history
            .iter()
            .rev()
            .map(|item| item.trim())
            .find(|item| !item.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| "暂无最新世界变化".to_string())
    }

    pub fn apply_fate_frame(
        &mut self,
        frame: &FateFrame,
        protagonist_name: &str,
        phase: TurnPhase,
        action_text: Option<&str>,
    ) {
        if !frame.location.trim().is_empty() {
            self.current_location = frame.location.trim().to_string();
        }

        let current_scene = [
            frame.scene_title.trim(),
            frame.description.trim(),
            frame.current_event.trim(),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
        if !current_scene.is_empty() {
            self.current_scene = current_scene;
        }

        let protagonist_name = protagonist_name.trim();
        let mut protagonist_state = String::new();
        let mut npcs_state = Vec::new();

        for character in &frame.characters {
            let entry = format_character_state(character);
            if entry.is_empty() {
                continue;
            }

            if !protagonist_name.is_empty() && character.name.trim() == protagonist_name {
                protagonist_state = entry;
            } else {
                npcs_state.push(entry);
            }
        }

        self.npcs_state = npcs_state;

        if !protagonist_state.is_empty() {
            self.protagonist_state = protagonist_state;
        } else if !frame.current_event.trim().is_empty() {
            self.protagonist_state = frame.current_event.trim().to_string();
        }

        let phase_label = match phase {
            TurnPhase::FateWeaving => "场景编排",
            TurnPhase::FateConsequence => "后果推演",
            _ => "命运推演",
        };

        let action_prefix = action_text
            .map(str::trim)
            .filter(|action| !action.is_empty())
            .map(|action| format!("主角行动：{action}\n"))
            .unwrap_or_default();

        let summary = frame.summary();
        if !summary.trim().is_empty() {
            self.history.push(format!("{phase_label}\n{action_prefix}{summary}"));
        }
    }

    fn format_list(items: &[String], fallback: &str) -> String {
        let lines = items
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>();

        if lines.is_empty() {
            fallback.to_string()
        } else {
            lines.join("\n")
        }
    }

    fn format_scalar(value: &str, fallback: &str) -> String {
        let value = value.trim();
        if value.is_empty() {
            fallback.to_string()
        } else {
            value.to_string()
        }
    }
}

fn format_character_state(character: &FateCharacterState) -> String {
    let name = character.name.trim();
    let deltas = character
        .status_delta
        .iter()
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();

    let mut parts = Vec::new();
    if !deltas.is_empty() {
        parts.push(format!("变化：{}", deltas.join("；")));
    }

    match (name.is_empty(), parts.is_empty()) {
        (true, true) => String::new(),
        (false, true) => name.to_string(),
        (true, false) => parts.join("；"),
        (false, false) => format!("{name}：{}", parts.join("；")),
    }
}
