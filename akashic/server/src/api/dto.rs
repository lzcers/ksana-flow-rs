use akashic_ecs::resources::{
    export::TaskView,
    history::RoundHistoryEntry,
    protagonist_action::{PendingProtagonistChoice, PlayerActionInput},
    turn_state::TurnPhase,
    world_snapshot::{ItemState, NpcState, OngoingEvent, WorldSnapshot},
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateProfilesData {
    pub world: String,
    pub protagonist: String,
    pub key_story_beats: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSessionRequest {
    pub world_profile: String,
    pub protagonist_profile: String,
    #[serde(default)]
    pub key_story_beats: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSessionData {
    pub session_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSaveSlotRequest {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSaveSlotData {
    pub slot_id: String,
    pub session_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveExportData {
    pub session_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub compressed_archive: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadArchiveRequest {
    pub compressed_archive: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadGameSessionRequest {
    pub slot_id: String,
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
    pub world_state: WorldStateData,
    pub history: Vec<RoundHistoryData>,
    pub current_task: Option<TaskView>,
    pub tasks: Vec<TaskView>,
    pub latest_narration: String,
    pub current_protagonist_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundHistoryData {
    pub round: u64,
    pub world_state: Option<WorldStateData>,
    pub narration_text: String,
    pub choices: Vec<PendingProtagonistChoice>,
    pub committed_action: Option<String>,
    pub selected_choice_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldStateData {
    pub round: u64,
    pub scene_title: String,
    pub time_absolute: String,
    pub time_relative: Option<String>,
    pub location_name: String,
    pub location_exits: Vec<String>,
    pub location_status: String,
    pub description: String,
    pub current_event: String,
    pub new_info: Vec<String>,
    pub inner_conflict: String,
    pub hard_anchors: Vec<String>,
    pub pace: String,
    pub atmosphere: String,
    pub focal_point: String,
    pub protagonist_condition: String,
    pub protagonist_known_secrets: Vec<String>,
    pub npcs: Vec<NpcStateData>,
    pub items: Vec<ItemStateData>,
    pub events_in_progress: Vec<OngoingEventData>,
    pub unsolved_threads: Vec<String>,
    pub pacing_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcStateData {
    pub name: String,
    pub location: String,
    pub mood: String,
    pub attitude: String,
    pub goal: String,
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStateData {
    pub name: String,
    pub location: String,
    pub status: String,
    pub awareness: String,
    pub relevance: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OngoingEventData {
    pub name: String,
    pub status: String,
    pub escalation_trigger: String,
}

impl From<WorldSnapshot> for WorldStateData {
    fn from(value: WorldSnapshot) -> Self {
        Self {
            round: value.round,
            scene_title: value.scene_title,
            time_absolute: value.time_absolute,
            time_relative: value.time_relative,
            location_name: value.location_name,
            location_exits: value.location_exits,
            location_status: value.location_status,
            description: value.description,
            current_event: value.current_event,
            new_info: value.new_info,
            inner_conflict: value.inner_conflict,
            hard_anchors: value.hard_anchors,
            pace: value.pace,
            atmosphere: value.atmosphere,
            focal_point: value.focal_point,
            protagonist_condition: value.protagonist_condition,
            protagonist_known_secrets: value.protagonist_known_secrets,
            npcs: value.npcs.into_iter().map(Into::into).collect(),
            items: value.items.into_iter().map(Into::into).collect(),
            events_in_progress: value
                .events_in_progress
                .into_iter()
                .map(Into::into)
                .collect(),
            unsolved_threads: value.unsolved_threads,
            pacing_note: value.pacing_note,
        }
    }
}

impl From<RoundHistoryEntry> for RoundHistoryData {
    fn from(value: RoundHistoryEntry) -> Self {
        let selected_choice_text = value.committed_action.as_ref().and_then(|action| {
            value.choices.iter().find_map(|choice| {
                (choice.option.action == *action).then(|| choice.option.title.clone())
            })
        });

        Self {
            round: value.round,
            world_state: value.world_snapshot.map(Into::into),
            narration_text: value.narration_text.unwrap_or_default(),
            choices: value.choices,
            committed_action: value.committed_action,
            selected_choice_text,
        }
    }
}

impl From<NpcState> for NpcStateData {
    fn from(value: NpcState) -> Self {
        Self {
            name: value.name,
            location: value.location,
            mood: value.mood,
            attitude: value.attitude,
            goal: value.goal,
            secrets: value.secrets,
        }
    }
}

impl From<ItemState> for ItemStateData {
    fn from(value: ItemState) -> Self {
        Self {
            name: value.name,
            location: value.location,
            status: value.status,
            awareness: value.awareness,
            relevance: value.relevance,
        }
    }
}

impl From<OngoingEvent> for OngoingEventData {
    fn from(value: OngoingEvent) -> Self {
        Self {
            name: value.name,
            status: value.status,
            escalation_trigger: value.escalation_trigger,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlGameSessionData {
    pub action: String,
}
