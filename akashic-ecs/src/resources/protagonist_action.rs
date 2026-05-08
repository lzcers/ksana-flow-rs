use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

/// 玩家 / 主角 Agent 做出的行动，等待世界 Agent 消费
#[derive(Resource, Debug, Clone)]
pub struct ProtagonistAction(pub String);

impl ProtagonistAction {
    pub fn take(&mut self) -> String {
        std::mem::replace(&mut self.0, "continue".to_string())
    }
    pub fn set(&mut self, action: String) {
        self.0 = action;
    }
}

impl Default for ProtagonistAction {
    fn default() -> Self {
        Self("continue".to_string())
    }
}

/// 主角 Agent 返回给玩家的一组选项。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ProtagonistOptions {
    #[serde(default)]
    pub options: Vec<ProtagonistOption>,
}

impl ProtagonistOptions {
    pub fn first_action(&self) -> Option<&str> {
        self.options.first().map(|option| option.action.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }
}

/// 单个可供玩家选择的主角行动。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtagonistOption {
    pub title: String,
    pub action: String,
    pub motivation_and_risk: String,
}
