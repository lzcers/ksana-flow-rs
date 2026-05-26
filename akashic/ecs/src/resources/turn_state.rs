use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    Idle,                 // 静止等待外部控制
    TurnReady,            // 已收到推进命令，允许启动本轮
    FateRunning,          // 命运任务执行中
    NarrationReady,       // 叙事任务待启动
    NarrationRunning,     // 叙事任务执行中
    ProtagonistReady,     // 主角任务待启动
    ProtagonistRunning,   // 主角任务执行中
    AwaitingPlayerChoice, // 等待外部玩家提交选择
    TurnComplete,         // 轮次完成，等待继续控制
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
        self.phase = TurnPhase::TurnComplete;
    }

    pub fn advance(&mut self) {
        match self.phase {
            TurnPhase::Idle => {
                if self.active_turn_id <= self.turn_index {
                    self.active_turn_id = self.turn_index + 1;
                }
                self.phase = TurnPhase::TurnReady;
            }
            TurnPhase::TurnComplete => {
                self.active_turn_id = self.turn_index + 1;
                self.phase = TurnPhase::TurnReady;
            }
            _ => {}
        }
    }
}
