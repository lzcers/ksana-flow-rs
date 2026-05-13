use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

/// 主角决策状态：既保存已确认动作，也保存当前等待外部确认的候选项。
#[derive(Resource, Debug, Clone)]
pub struct ProtagonistDecisionState {
    committed_action: String,
    choices: Vec<PendingProtagonistChoice>,
}

impl ProtagonistDecisionState {
    pub fn committed_action(&self) -> &str {
        &self.committed_action
    }

    pub fn choices(&self) -> &[PendingProtagonistChoice] {
        &self.choices
    }

    pub fn first_choice_id(&self) -> Option<&str> {
        self.choices.first().map(|choice| choice.id.as_str())
    }

    pub fn replace_with_options(&mut self, options: ProtagonistOptions) {
        self.choices = options
            .options
            .into_iter()
            .enumerate()
            .map(|(index, option)| PendingProtagonistChoice {
                id: format!("choice-{}", index + 1),
                option,
            })
            .collect();
    }

    pub fn clear_choices(&mut self) {
        self.choices.clear();
    }

    pub fn commit_selection(&mut self, selection: &str) -> String {
        let action = self.find_action(selection).unwrap_or(selection).to_string();
        self.choices.clear();
        self.committed_action = action.clone();
        action
    }

    pub fn find_action(&self, choice_id: &str) -> Option<&str> {
        self.choices
            .iter()
            .find(|choice| choice.id == choice_id)
            .map(|choice| choice.option.action.as_str())
    }
}

impl Default for ProtagonistDecisionState {
    fn default() -> Self {
        Self {
            committed_action: "start".to_string(),
            choices: Vec::new(),
        }
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

/// 可供外部玩家提交的主角候选项。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingProtagonistChoice {
    pub id: String,
    pub option: ProtagonistOption,
}
