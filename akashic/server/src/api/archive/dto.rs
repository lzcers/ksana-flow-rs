use serde::{Deserialize, Serialize};

use agent::agent::context::Context;
use akashic_ecs::resources::{
    history::SessionHistoryLog,
    protagonist_action::PendingProtagonistChoice, turn_state::TurnPhase,
    world_snapshot::WorldSnapshot,
};

/// 内部恢复用：TurnState 的可序列化快照
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TurnStateArchive {
    pub phase: TurnPhase,
    pub turn_index: u64,
    pub active_turn_id: u64,
}

/// 内部恢复用：主角决策状态快照
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ProtagonistDecisionArchive {
    pub committed_action: String,
    pub choices: Vec<PendingProtagonistChoice>,
}

/// 整个 session 的内部归档载荷
/// 这是恢复真源，不是面向前端的展示 DTO。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionArchivePayload {
    /// 复用旧 session_id
    pub session_id: String,

    /// 存档展示标题，用于列表页
    pub title: String,

    /// 原始文本资料，按你的要求只存文本
    pub world_profile: String,
    pub protagonist_profile: String,

    /// 当前回合状态
    pub turn_state: TurnStateArchive,

    /// 三个 Entity 的完整 Context，保证 LLM 对话连续性
    pub fate_weaver: Context,
    pub upper_narrator: Context,
    pub protagonist: Context,
    /// 当前世界状态
    pub world_snapshot: WorldSnapshot,

    /// 当前主角决策状态，保证选项可继续提交
    pub protagonist_decision: ProtagonistDecisionArchive,

    /// 每轮结构化历史，保证前端可恢复完整时间线
    pub history_log: SessionHistoryLog,
}

/// 数据库存档槽记录
/// 用于“多存档槽 + 列表筛选字段”。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SaveSlotRecord {
    pub slot_id: String,
    pub session_id: String,
    pub title: String,
    /// ISO8601 文本即可，避免过早引入额外时间类型耦合
    pub created_at: String,
    pub updated_at: String,

    /// 真正的完整归档
    pub payload: SessionArchivePayload,
}
