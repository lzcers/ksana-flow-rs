use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpperNarratorRequest {
    pub round: u32,
    pub chapter: String,
    pub section: String,
    pub compact_context: String,
    pub event: String,
    pub situation: String,
    pub info_gained: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpperNarration {
    pub round: u32,
    pub title: String,
    pub content: String,
}
