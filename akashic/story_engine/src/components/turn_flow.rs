use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TurnFlow {
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub stage: TurnStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStage {
    Idle,
    TurnReady,
    FateRunning,
    NarrationReady,
    NarrationRunning,
    ProtagonistReady,
    ProtagonistRunning,
    AwaitingPlayerChoice,
    TurnComplete,
    StoryEnded,
    Failed,
}

impl Default for TurnStage {
    fn default() -> Self {
        Self::Idle
    }
}

impl Default for TurnFlow {
    fn default() -> Self {
        Self {
            turn_index: 0,
            active_turn_id: 0,
            stage: TurnStage::Idle,
        }
    }
}

impl TurnFlow {
    pub fn reset(&mut self, next_turn_id: Option<u64>) {
        self.stage = TurnStage::Idle;
        self.active_turn_id = next_turn_id.unwrap_or_else(|| self.turn_index + 1);
    }

    pub fn finish_turn(&mut self) {
        self.turn_index = self.active_turn_id.max(self.turn_index + 1);
        self.active_turn_id = self.turn_index;
        self.stage = TurnStage::TurnComplete;
    }

    pub fn finish_story(&mut self) {
        self.turn_index = self.active_turn_id.max(self.turn_index + 1);
        self.active_turn_id = self.turn_index;
        self.stage = TurnStage::StoryEnded;
    }

    pub fn advance(&mut self) {
        match self.stage {
            TurnStage::Idle => {
                if self.active_turn_id <= self.turn_index {
                    self.active_turn_id = self.turn_index + 1;
                }
                self.stage = TurnStage::TurnReady;
            }
            TurnStage::TurnComplete => {
                self.active_turn_id = self.turn_index + 1;
                self.stage = TurnStage::TurnReady;
            }
            _ => {}
        }
    }
}
