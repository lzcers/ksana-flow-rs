use bevy_ecs::component::Component;

pub mod fate_weaver;
pub mod protagonist;
pub mod upper_narrator;

// 整个故事的状态驱动器
#[derive(Component)]
pub enum AkashicState {
    Idle,                 // 初始状态
    FateWaeving,          // 命运编织中
    FateWeavingCompleted, // 命运编织完成
    AwaitingProtagonist,  // 等待主角行动
    ProtagonistCompleted, // 主角行动完成
    RoundCompleted,       // 一轮完成交互完成
    AwatingNarrotor,      // 等待叙事完成
    NarratorCompleted,    // 故事推演完成
}
