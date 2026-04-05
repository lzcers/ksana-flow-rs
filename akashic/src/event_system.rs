use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldFact {
    #[serde(rename = "type")]
    pub event_type: String,
    pub description: String,
    pub participants: Vec<String>,
    pub impact: String,
    pub cause: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NarrativeConstraints {
    pub tone: String,
    pub focus: String,
    pub length_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundFacts {
    pub round: u32,
    pub phase: String,
    pub progress: f32,
    pub events: Vec<WorldFact>,
    pub world_state_delta: String,
    pub character_state_delta: String,
    pub pacing_instruction: String,
    pub narrative_constraints: NarrativeConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldSnapshot {
    pub current_location: String,
    pub surroundings: String,
    pub present_npcs: Vec<String>,
    pub available_resources: String,
    pub active_threats: String,
    pub recent_events: String,
    pub emotional_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FateRoundEnvelope {
    pub facts: RoundFacts,
    pub world_snapshot: WorldSnapshot,
    #[serde(default)]
    pub should_end: bool,
    #[serde(default)]
    pub ending_hint: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    StartStory,
    RoundPrepared {
        round: u32,
        envelope: FateRoundEnvelope,
    },
    NarrativeChunk {
        round: u32,
        content: String,
    },
    NarrativeRendered {
        round: u32,
        narrative: String,
    },
    ProtagonistResponded {
        round: u32,
        response: ProtagonistResponse,
    },
    DecisionAutoSelected {
        round: u32,
        option_id: String,
        reason: String,
    },
    StoryCompleted {
        round: u32,
        ending_hint: Option<String>,
        narrative: String,
    },
    AgentError {
        agent: String,
        message: String,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct EventChannel {
    channel: broadcast::Sender<Event>,
}

impl EventChannel {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<Event>(64);
        Self { channel: tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.channel.subscribe()
    }

    pub fn send(&self, event: Event) {
        let _ = self.channel.send(event);
    }
}
