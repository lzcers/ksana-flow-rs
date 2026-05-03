use std::collections::HashMap;

use crate::{prompts::fate_weaver_prompt, resources::world_state::WorldState, utils::build_layer};
use agent::agent::{Context, LayerKind};
use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};
use serde_json::json;
// 标识 Entity
#[derive(Component)]
pub struct FateWeaver {
    world_profile: String,
    protagonist_profile: String,
    protagonist_name: String,
}

impl FateWeaver {
    const RECENT_HISTORY_WINDOW: usize = 3;
    const HISTORY_SUMMARY_ENTRIES: usize = 6;

    pub fn new(world_profile: &str, prota_profile: &str) -> Self {
        Self {
            world_profile: world_profile.to_string(),
            protagonist_profile: prota_profile.to_string(),
            protagonist_name: extract_protagonist_name(prota_profile),
        }
    }

    pub fn protagonist_name(&self) -> &str {
        &self.protagonist_name
    }

    pub fn build_scene_context(&self, world_state: &WorldState) -> Context {
        let prompt = fate_weaver_prompt::SCENE_SYSTEM_PROMPT
            .replace("{world_profile}", &self.world_profile)
            .replace("{protagonist_profile}", &self.protagonist_profile)
            .replace("{output_schema}", fate_weaver_prompt::OUTPUT_SCHEMA);

        self.build_task_context(prompt, self.build_scene_memory_items(world_state))
    }

    pub fn build_consequence_context(&self, world_state: &WorldState, action: &str) -> Context {
        let prompt = fate_weaver_prompt::CONSEQUENCE_SYSTEM_PROMPT
            .replace("{world_profile}", &self.world_profile)
            .replace("{protagonist_profile}", &self.protagonist_profile)
            .replace("{output_schema}", fate_weaver_prompt::OUTPUT_SCHEMA);

        self.build_task_context(
            prompt,
            self.build_consequence_memory_items(world_state, action),
        )
    }

    fn build_task_context(&self, prompt: String, memory_items: Vec<String>) -> Context {
        let mut context = Context::new().layer(build_layer(
            "fate-weaver-task",
            LayerKind::System,
            json!(prompt),
            100,
        ));

        if !memory_items.is_empty() {
            let memory_entries = memory_items
                .into_iter()
                .map(|content| json!({ "content": content }))
                .collect::<Vec<_>>();
            context = context.layer(build_layer(
                "fate-weaver-runtime",
                LayerKind::Memory,
                json!(memory_entries),
                50,
            ));
        }

        context
    }

    fn build_scene_memory_items(&self, world_state: &WorldState) -> Vec<String> {
        vec![
            self.build_history_summary_item(world_state),
            self.build_recent_history_item(world_state),
            self.build_world_state_item(world_state),
        ]
    }

    fn build_consequence_memory_items(
        &self,
        world_state: &WorldState,
        action: &str,
    ) -> Vec<String> {
        let mut items = self.build_scene_memory_items(world_state);
        let action = action.trim();
        if !action.is_empty() {
            items.push(format!("主角行动\n{action}"));
        }
        items
    }

    fn build_history_summary_item(&self, world_state: &WorldState) -> String {
        format!(
            "世界历史摘要\n{}",
            world_state
                .history_summary_text(Self::RECENT_HISTORY_WINDOW, Self::HISTORY_SUMMARY_ENTRIES,)
        )
    }

    fn build_recent_history_item(&self, world_state: &WorldState) -> String {
        format!(
            "最近世界历史窗口\n{}",
            world_state.recent_history_window_text(Self::RECENT_HISTORY_WINDOW)
        )
    }

    fn build_world_state_item(&self, world_state: &WorldState) -> String {
        format!(
            "当前状态\n位置：{}\n场景：{}\nNPC状态：\n{}\n主角状态：\n{}",
            world_state.current_location_text(),
            world_state.current_scene_text(),
            world_state.npcs_state_text(),
            world_state.protagonist_state_text(),
        )
    }
}

/// Fate 阶段产出的规范化场景快照，用于后续叙事、主角决策与上下文回写。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateFrame {
    /// 当前命运片段的场景标题，兼容旧字段 `section`。
    #[serde(default, alias = "section")]
    pub scene_title: String,
    /// 场景发生的时间或时间阶段。
    #[serde(default)]
    pub time: String,
    /// 场景发生的地点。
    #[serde(default)]
    pub location: String,
    /// 对环境氛围或场景状态的整体描述，兼容旧字段 `environment`。
    #[serde(default, alias = "environment")]
    pub description: String,
    /// 本场景中涉及的人物状态变化列表。
    #[serde(default)]
    pub characters: Vec<FateCharacterState>,
    /// 当前正在推进的核心事件，兼容旧字段 `event`。
    #[serde(default, alias = "event")]
    pub current_event: String,
    /// 本次命运推进中新获得的情报或认知。
    #[serde(default)]
    pub new_info: Vec<String>,
    /// 本段推进已完成的关键里程碑。
    #[serde(default)]
    pub milestones_completed: Vec<String>,
    /// 场景节奏与推进态势的结构化描述。
    #[serde(default)]
    pub pacing: FatePacing,
    /// 提供给主角或外部系统的可选行动分支。
    #[serde(default)]
    pub choices: Vec<FateChoice>,
}

impl FateFrame {
    pub fn brief_summary(&self) -> String {
        [self.scene_title.as_str(), self.current_event.as_str()]
            .into_iter()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();

        Self::push_field(&mut lines, "场景", &self.scene_title);

        let time_location = [self.time.as_str(), self.location.as_str()]
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" @ ");
        if !time_location.is_empty() {
            lines.push(format!("时空：{}", time_location));
        }

        Self::push_field(&mut lines, "描述", &self.description);
        Self::push_field(&mut lines, "事件", &self.current_event);

        let character_lines = self
            .characters
            .iter()
            .filter_map(|character| {
                let name = character.name.trim();
                let mut detail_parts = Vec::new();

                if !character.status_delta.is_empty() {
                    let mut deltas = character
                        .status_delta
                        .iter()
                        .map(|(key, value)| (key.trim(), value.trim()))
                        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>();
                    deltas.sort();
                    if !deltas.is_empty() {
                        detail_parts.push(format!("变化：{}", deltas.join("；")));
                    }
                }

                if name.is_empty() && detail_parts.is_empty() {
                    None
                } else if detail_parts.is_empty() {
                    Some(format!("- {}", name))
                } else if name.is_empty() {
                    Some(format!("- {}", detail_parts.join("；")))
                } else {
                    Some(format!("- {}：{}", name, detail_parts.join("；")))
                }
            })
            .collect::<Vec<_>>();
        if !character_lines.is_empty() {
            lines.push("人物：".to_string());
            lines.extend(character_lines);
        }

        let info_lines = self
            .new_info
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| format!("- {}", item))
            .collect::<Vec<_>>();
        if !info_lines.is_empty() {
            lines.push("情报：".to_string());
            lines.extend(info_lines);
        }

        let milestone_lines = self
            .milestones_completed
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| format!("- {}", item))
            .collect::<Vec<_>>();
        if !milestone_lines.is_empty() {
            lines.push("里程碑：".to_string());
            lines.extend(milestone_lines);
        }

        let pacing = [
            ("节奏", self.pacing.beat.trim()),
            ("张力", self.pacing.tension.trim()),
        ]
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(label, value)| format!("{label}：{value}"))
        .collect::<Vec<_>>()
        .join("；");
        if !pacing.is_empty() {
            lines.push(format!("推进：{}", pacing));
        }

        let choice_lines = self
            .choices
            .iter()
            .filter_map(|choice| {
                let id = choice.id.trim();
                let text = choice.text.trim();
                let trigger = choice.trigger.trim();

                match (id.is_empty(), text.is_empty(), trigger.is_empty()) {
                    (false, false, false) => Some(format!("- {}：{} -> {}", id, text, trigger)),
                    (true, false, false) => Some(format!("- {} -> {}", text, trigger)),
                    (false, false, true) => Some(format!("- {}：{}", id, text)),
                    (true, false, true) => Some(format!("- {}", text)),
                    (false, true, false) => Some(format!("- {} -> {}", id, trigger)),
                    (false, true, true) => Some(format!("- {}", id)),
                    (true, true, false) => Some(format!("- {}", trigger)),
                    (true, true, true) => None,
                }
            })
            .collect::<Vec<_>>();
        if !choice_lines.is_empty() {
            lines.push("选择：".to_string());
            lines.extend(choice_lines);
        }

        lines.join("\n")
    }

    fn push_field(lines: &mut Vec<String>, label: &str, value: &str) {
        let value = value.trim();
        if !value.is_empty() {
            lines.push(format!("{label}：{value}"));
        }
    }
}

fn extract_protagonist_name(profile: &str) -> String {
    profile
        .lines()
        .find_map(|line| line.trim().strip_prefix("姓名："))
        .map(|name| {
            name.trim()
                .split('/')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateCharacterState {
    pub name: String,
    #[serde(default, alias = "deltas")]
    pub status_delta: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FatePacing {
    #[serde(default)]
    pub beat: String,
    #[serde(default)]
    pub tension: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FateChoice {
    pub id: String,
    pub text: String,
    #[serde(default, alias = "next_trigger")]
    pub trigger: String,
}
