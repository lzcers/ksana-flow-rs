use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export Context types from context module
pub use crate::agents::context::{Context, Layer, LayerKind, LayerMeta};
use crate::core::Message;

// ============================================================================
// AgentState: Agent 状态
// ============================================================================
/// Agent 状态 - 可持久化、可追踪的执行实体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentState {
    // === 标识信息 ===
    /// 唯一 ID
    pub job_id: Uuid,
    /// 所属用户
    pub user_id: String,
    /// 关联会话
    pub conversation_id: Option<Uuid>,

    // === 任务元数据 ===
    /// 标题
    #[serde(default)]
    pub title: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 分类
    pub category: Option<String>,

    // === 状态管理 ===
    /// 当前状态
    #[serde(default)]
    pub state: JobState,

    // === 资源追踪 ===
    /// 预算
    pub budget: Option<Decimal>,
    /// 实际成本
    #[serde(default)]
    pub actual_cost: Decimal,

    // === 执行上下文 ===
    /// 分层上下文（对话、记忆、人格等）
    #[serde(default)]
    pub context: Context,

    // === 执行状态 ===
    /// 当前迭代次数
    #[serde(default)]
    pub iteration: usize,
    /// 最大迭代
    pub max_iterations: usize,
}

impl AgentState {
    pub fn get_message(&self) -> Vec<Message> {
        self.context.to_messages()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JobState {
    /// 待执行
    #[default]
    Pending,
    /// 执行中
    Running,
    /// 等待输入
    WaitingInput,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}
