use serde::{Deserialize, Serialize};

pub type TurnPhase = crate::components::turn_flow::TurnStage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TurnState {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
}
