use bevy_ecs::{entity::Entity, message::Message};

use crate::resources::{protagonist_action::PlayerActionInput, turn_state::TurnPhase};

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    SubmitPlayerAction { turn_id: u64, input: PlayerActionInput },
}

#[derive(Message, Debug, Clone)]
pub enum TurnControl {
    StartNextTurn,
    ResetToIdle { next_turn_id: Option<u64> },
}

#[derive(Message, Debug, Clone)]
pub enum TurnEvent {
    FateStarted,        // 命运任务已启动
    FateCompleted,      // 命运任务已完成
    NarrationStarted,   // 叙事任务已启动
    NarrationCompleted, // 叙事任务已完成
    ProtagonistStarted, // 主角任务已启动
    ChoicesPrepared,    // 已生成候选项，等待外部提交
    DecisionCommitted,  // 玩家或自动选择已提交
    TaskFailed {
        turn_id: u64,
        stage: TurnPhase,
        entity: Entity,
        message: String,
    },
}
