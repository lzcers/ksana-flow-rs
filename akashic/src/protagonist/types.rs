use serde::{Deserialize, Serialize};

use crate::fate_weaver::types::Choice;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistActionRequest {
    pub round: u32,
    pub compact_context: String,
    pub situation: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistDecision {
    pub choice_id: String,
    pub action: String,
    pub rationale: String,
}
