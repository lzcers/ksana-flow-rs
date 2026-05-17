use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    Idle,                 // 静止等待外部控制
    Ready,                // 已收到推进命令，允许启动本轮
    FateWeaving,          // 命运编织状态
    NarratorWriting,      // 故事编写状态
    NarratorStory,        // 故事编排状态
    ProtagonistAction,    // 主角行动阶段
    AwaitingProtagonist,  // 等待主角状态
    AwaitingPlayerChoice, // 等待外部玩家提交选择
    TurnFinished,         // 轮次结束状态
    Failed,
}

impl Default for TurnPhase {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Resource, Debug)]
pub struct TurnState {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            phase: TurnPhase::Idle,
            turn_index: 0,
            active_turn_id: 0,
        }
    }
}

impl TurnState {
    pub fn reset(&mut self, next_turn_id: Option<u64>) {
        self.phase = TurnPhase::Idle;
        self.active_turn_id = next_turn_id.unwrap_or_else(|| self.turn_index + 1);
    }

    pub fn finish_turn(&mut self) {
        self.turn_index = self.active_turn_id.max(self.turn_index + 1);
        self.active_turn_id = self.turn_index;
        self.phase = TurnPhase::TurnFinished;
    }

    pub fn advance(&mut self) {
        match self.phase {
            TurnPhase::Idle => {
                if self.active_turn_id <= self.turn_index {
                    self.active_turn_id = self.turn_index + 1;
                }
                self.phase = TurnPhase::Ready;
            }
            TurnPhase::TurnFinished => {
                self.active_turn_id = self.turn_index + 1;
                self.phase = TurnPhase::Ready;
            }
            _ => {}
        }
    }
}
