use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TurnFlow {
    pub turn_index: u64,
    pub active_turn_id: u64,
    pub stage: TurnStage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStage {
    #[default]
    Idle,
    Simulation,
    Application,
    AwaitingPlayer,
    TurnCompleted,
    Ended,
    Failed,
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
        self.stage = TurnStage::TurnCompleted;
    }

    pub fn end(&mut self) {
        self.turn_index = self.active_turn_id.max(self.turn_index + 1);
        self.active_turn_id = self.turn_index;
        self.stage = TurnStage::Ended;
    }

    pub fn advance(&mut self) {
        match self.stage {
            TurnStage::Idle => {
                if self.active_turn_id <= self.turn_index {
                    self.active_turn_id = self.turn_index + 1;
                }
                self.stage = TurnStage::Simulation;
            }
            TurnStage::TurnCompleted => {
                self.active_turn_id = self.turn_index + 1;
                self.stage = TurnStage::Simulation;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_and_finishes_turn_without_reusing_turn_ids() {
        let mut flow = TurnFlow::default();

        flow.advance();
        assert_eq!(flow.active_turn_id, 1);
        assert_eq!(flow.stage, TurnStage::Simulation);

        flow.finish_turn();
        assert_eq!(flow.turn_index, 1);
        assert_eq!(flow.active_turn_id, 1);
        assert_eq!(flow.stage, TurnStage::TurnCompleted);

        flow.advance();
        assert_eq!(flow.active_turn_id, 2);
        assert_eq!(flow.stage, TurnStage::Simulation);
    }

    #[test]
    fn accepts_only_current_stage_names() {
        assert_eq!(
            serde_json::from_str::<TurnStage>("\"awaiting_player\"").unwrap(),
            TurnStage::AwaitingPlayer
        );
        assert!(serde_json::from_str::<TurnStage>("\"simulation_ready\"").is_err());
        assert!(serde_json::from_str::<TurnStage>("\"awaiting_player_choice\"").is_err());
        assert!(serde_json::from_str::<TurnStage>("\"story_ended\"").is_err());
    }
}
