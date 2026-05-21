use akashic_ecs::resources::{
    export::TaskView, protagonist_action::PendingProtagonistChoice, turn_state::TurnPhase,
    world_snapshot::WorldSnapshot,
};
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

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateProfilesRequest {
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateProfilesData {
    pub world: String,
    pub protagonist: String,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionChoiceInput {
    pub choice_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameSessionControlCommand {
    Continue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlGameSessionRequest {
    pub control: Option<GameSessionControlCommand>,
    pub choice: Option<SessionChoiceInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSessionWorldStateData {
    pub session_id: String,
    pub status: String,
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub world_state: WorldSnapshot,
    pub current_task: Option<TaskView>,
    pub tasks: Vec<TaskView>,
    pub latest_narration: String,
    pub current_protagonist_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlGameSessionData {
    pub action: String,
}
