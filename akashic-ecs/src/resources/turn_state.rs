use bevy_ecs::resource::Resource;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Idle,
    FateWeaving,
    NarratorScene,
    AwaitingProtagonist,
    FateConsequence,
    NarratorStory,
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
    // 仅用于当前回合阶段间广播摘要，不作为任何 Agent 决策输入。
    pub latest_broadcast_summary: String,
    pub latest_protagonist_action: String,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            phase: TurnPhase::Idle,
            turn_index: 0,
            active_turn_id: 1,
            latest_broadcast_summary: String::new(),
            latest_protagonist_action: String::new(),
        }
    }
}

impl TurnState {
    pub fn start_turn(&mut self, turn_id: u64) {
        self.active_turn_id = turn_id;
        self.phase = TurnPhase::FateWeaving;
        self.latest_broadcast_summary.clear();
        self.latest_protagonist_action.clear();
    }

    pub fn reset(&mut self, next_turn_id: Option<u64>) {
        self.phase = TurnPhase::Idle;
        self.active_turn_id = next_turn_id.unwrap_or_else(|| self.turn_index + 1);
        self.latest_broadcast_summary.clear();
        self.latest_protagonist_action.clear();
    }

    pub fn finish_turn(&mut self) {
        self.turn_index += 1;
        self.active_turn_id = self.turn_index + 1;
        self.phase = TurnPhase::Idle;
        self.latest_broadcast_summary.clear();
        self.latest_protagonist_action.clear();
    }
}
