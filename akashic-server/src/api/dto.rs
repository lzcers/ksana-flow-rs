use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionPath {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: String,
    pub gender: String,
    pub age: u32,
    pub appearance: String,
    pub traits: CharacterTraits,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTraits {
    pub courage: u32,
    pub rationality: u32,
    pub altruism: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub era: String,
    pub core_conflict: String,
    pub special_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceCostHints {
    pub intuition: u32,
    pub obsession: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub disabled: bool,
    pub cost_hints: ChoiceCostHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryNode {
    pub id: String,
    pub text: String,
    pub image: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStateView {
    pub game_state: String,
    pub phase: String,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub current_location: String,
    pub current_scene: String,
    pub protagonist_state: String,
    pub npcs_state: String,
    pub latest_history: String,
    pub latest_broadcast_summary: String,
    pub latest_protagonist_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResources {
    pub obsession_points: i32,
    pub intuition_points: i32,
    pub days_left: i32,
    pub world_news: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingTurningPoint {
    pub cause: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingData {
    pub biography: String,
    pub turning_points: Vec<EndingTurningPoint>,
    pub legacy: String,
    pub cgs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDelta {
    pub obsession_points: i32,
    pub intuition_points: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGameSessionRequest {
    pub character: Character,
    pub world: World,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSessionData {
    pub session_id: String,
    pub created_at: String,
    pub status: String,
    pub ending_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSessionSnapshot {
    pub session_id: String,
    pub status: String,
    pub character: Character,
    pub world: World,
    pub resources: SessionResources,
    pub current_node: StoryNode,
    pub state_view: RuntimeStateView,
    pub ending_status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitChoiceRequest {
    pub choice_id: String,
    pub use_obsession: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitChoiceData {
    pub accepted: bool,
    pub session_id: String,
    pub turn_id: u64,
    pub resource_delta: ResourceDelta,
    pub resources: SessionResources,
    pub state_view: RuntimeStateView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSessionEndingData {
    pub session_id: String,
    pub ending_status: String,
    pub ending: EndingData,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryListData {
    pub items: Vec<HistoryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub turn_index: u64,
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamHandshakeData {
    pub session_id: String,
    pub protocol: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthzData {
    pub status: String,
    pub service_name: String,
    pub api_version: String,
}
