pub mod payload {
    use super::parse::{DecisionOption, ProtagonistAction};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(tag = "type", content = "payload", rename_all = "snake_case")]
    /// 主角 Agent 对外广播的感知、决策与行动事件。
    pub enum ProtagonistEvent {
        /// 广播主角已经接收到当前轮的世界感知。
        WorldPerceived {
            round: u32,
            perception: ProtagonistPerception,
        },
        /// 广播命运编织者下发的常规决策框架。
        DecisionFrameReceived {
            round: u32,
            frame: DecisionFrame,
        },
        /// 广播命运编织者下发的终局决策框架。
        FinalDecisionFrameReceived {
            round: u32,
            frame: FinalDecisionFrame,
        },
        /// 广播主角为用户准备好的候选行动选项。
        OptionsPrepared {
            round: u32,
            options: Vec<ActionOption>,
            recommended_option_id: Option<String>,
        },
        /// 请求用户或外部编排器在多个选项中进行选择。
        DecisionRequested {
            round: u32,
            request: UserDecisionRequest,
        },
        /// 广播主角最终承诺执行的行动指令。
        ActionCommitted {
            round: u32,
            action: ActionCommand,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ActionRisk {
        Safe,
        Low,
        Medium,
        High,
        Deadly,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum CharacterFit {
        High,
        Medium,
        Low,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum DecisionConfidence {
        High,
        Medium,
        Low,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum DecisionImportance {
        Low,
        Medium,
        High,
        Critical,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum TimePressure {
        None,
        Low,
        Medium,
        High,
        Critical,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ProtagonistActionType {
        Move,
        Dialogue,
        Investigate,
        Combat,
        Rest,
        Craft,
        Trade,
        Other,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ProtagonistPerception {
        pub location: String,
        pub surroundings: String,
        pub present_npcs: Vec<String>,
        pub available_resources: Vec<String>,
        pub active_threats: Vec<String>,
        pub recent_events: Vec<String>,
        pub emotional_context: String,
        pub unknowns: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct DecisionFrame {
        pub objective: String,
        pub urgency: String,
        pub stakes: String,
        pub notable_uncertainties: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct FinalDecisionFrame {
        pub dilemma: String,
        pub stakes: String,
        pub options: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ActionOption {
        pub option_id: String,
        pub action: String,
        pub consequence_hint: String,
        pub risk_level: ActionRisk,
        pub character_fit: CharacterFit,
        pub protagonist_tendency: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct UserDecisionRequest {
        pub situation: String,
        pub inner_thought: String,
        pub options: Vec<ActionOption>,
        pub recommended_option_id: Option<String>,
        pub confidence: DecisionConfidence,
        pub importance: DecisionImportance,
        pub time_pressure: TimePressure,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ActionCommand {
        pub action_type: ProtagonistActionType,
        pub target: String,
        pub method: String,
        pub inner_monologue: String,
        pub expected_outcome: String,
    }

    impl ActionRisk {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "LOW" => Self::Low,
                "MEDIUM" => Self::Medium,
                "HIGH" => Self::High,
                "DEADLY" => Self::Deadly,
                _ => Self::Safe,
            }
        }
    }

    impl CharacterFit {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "高" | "HIGH" => Self::High,
                "低" | "LOW" => Self::Low,
                _ => Self::Medium,
            }
        }
    }

    impl TimePressure {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "LOW" => Self::Low,
                "MEDIUM" => Self::Medium,
                "HIGH" => Self::High,
                "CRITICAL" => Self::Critical,
                _ => Self::None,
            }
        }
    }

    impl ProtagonistActionType {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "DIALOGUE" => Self::Dialogue,
                "INVESTIGATE" => Self::Investigate,
                "COMBAT" => Self::Combat,
                "REST" => Self::Rest,
                "CRAFT" => Self::Craft,
                "TRADE" => Self::Trade,
                "OTHER" => Self::Other,
                _ => Self::Move,
            }
        }
    }

    impl ActionOption {
        pub fn from_decision_option(option: &DecisionOption) -> Self {
            Self {
                option_id: option.id.clone(),
                action: option.action.clone(),
                consequence_hint: option.consequence_hint.clone(),
                risk_level: ActionRisk::from_raw(&option.risk_level),
                character_fit: CharacterFit::from_raw(&option.character_fit),
                protagonist_tendency: option.protagonist_tendency.clone(),
            }
        }
    }

    impl ActionCommand {
        pub fn from_protagonist_action(action: ProtagonistAction) -> Self {
            Self {
                action_type: ProtagonistActionType::from_raw(&action.action_type),
                target: action.target,
                method: action.method,
                inner_monologue: action.inner_monologue,
                expected_outcome: action.expected_outcome,
            }
        }
    }
}

#[allow(unused_imports)]
pub use payload::{
    ActionCommand, ActionOption, ActionRisk, CharacterFit, DecisionConfidence, DecisionFrame,
    DecisionImportance, FinalDecisionFrame, ProtagonistActionType, ProtagonistEvent,
    ProtagonistPerception, TimePressure, UserDecisionRequest,
};

pub mod parse {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct DecisionOption {
        pub id: String,
        pub action: String,
        pub consequence_hint: String,
        pub risk_level: String,
        pub character_fit: String,
        pub protagonist_tendency: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct DecisionRequest {
        pub situation: String,
        pub inner_thought: String,
        pub options: Vec<DecisionOption>,
        pub time_pressure: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct ProtagonistAction {
        pub action_type: String,
        pub target: String,
        pub method: String,
        pub inner_monologue: String,
        pub expected_outcome: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct ProtagonistResponse {
        pub mode: String,
        #[serde(default)]
        pub decision_request: Option<DecisionRequest>,
        pub recommended_action: ProtagonistAction,
        #[serde(default)]
        pub recommended_option_id: Option<String>,
    }
}

#[allow(unused_imports)]
pub use parse::{DecisionOption, DecisionRequest, ProtagonistAction, ProtagonistResponse};
