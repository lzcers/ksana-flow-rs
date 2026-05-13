use bevy_ecs::resource::Resource;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Idle,                // 初始状态
    FateWeaving,         // 命运编织状态
    NarratorWriting,     // 故事编写状态
    NarratorStory,       // 故事编排状态
    ProtagonistAction,   // 主角行动阶段
    AwaitingProtagonist, // 等待主角状态
    AwaitingPlayerChoice, // 等待外部玩家提交选择
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
    pub fn start_turn(&mut self, turn_id: u64) {
        self.active_turn_id = turn_id;
        self.phase = TurnPhase::FateWeaving;
    }

    pub fn reset(&mut self, next_turn_id: Option<u64>) {
        self.phase = TurnPhase::Idle;
        self.active_turn_id = next_turn_id.unwrap_or_else(|| self.turn_index + 1);
    }

    pub fn finish_turn(&mut self) {
        self.turn_index += 1;
        self.active_turn_id = self.turn_index;
        self.phase = TurnPhase::Idle;
    }
}
