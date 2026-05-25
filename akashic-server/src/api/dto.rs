use akashic_ecs::resources::{
    export::TaskView,
    protagonist_action::{PendingProtagonistChoice, PlayerActionInput},
    turn_state::TurnPhase,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSessionRequest {
    pub world_profile: String,
    pub protagonist_profile: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSessionData {
    pub session_id: String,
    pub created_at: String,
}

pub type SessionActionInput = PlayerActionInput;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameSessionControlCommand {
    Continue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlGameSessionRequest {
    pub control: Option<GameSessionControlCommand>,
    pub action: Option<SessionActionInput>,
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
