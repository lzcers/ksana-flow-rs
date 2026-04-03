use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export Context types from context module
pub use crate::agent::context::{Context, Layer, LayerKind, LayerMeta};
use crate::core::{Message, Usage};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStatistics {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

impl TokenStatistics {
    pub fn add_usage(&mut self, usage: &Usage) {
        self.prompt_tokens += u64::from(usage.prompt_tokens);
        self.completion_tokens += u64::from(usage.completion_tokens);
        self.total_tokens += u64::from(usage.total_tokens);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTerminalReason {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineMetrics {
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_active_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub total_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetrics {
    #[serde(default)]
    pub iteration: usize,
    #[serde(default)]
    pub max_iterations: usize,
    #[serde(default)]
    pub terminal_reason: Option<AgentTerminalReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyMetrics {
    #[serde(default)]
    pub total_model_duration_ms: u64,
    #[serde(default)]
    pub total_tool_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolMetrics {
    #[serde(default)]
    pub call_count: u64,
    #[serde(default)]
    pub success_count: u64,
    #[serde(default)]
    pub failure_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorMetrics {
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metrics {
    #[serde(default)]
    pub timeline: TimelineMetrics,
    #[serde(default)]
    pub execution: ExecutionMetrics,
    #[serde(default)]
    pub tokens: TokenStatistics,
    #[serde(default)]
    pub latency: LatencyMetrics,
    #[serde(default)]
    pub tools: ToolMetrics,
    #[serde(default)]
    pub errors: ErrorMetrics,
}

impl Metrics {
    pub fn with_max_iterations(max_iterations: usize) -> Self {
        let mut metrics = Self::default();
        metrics.timeline.created_at = Some(Utc::now());
        metrics.execution.max_iterations = max_iterations;
        metrics
    }

    pub fn mark_started(&mut self) {
        let now = Utc::now();
        if self.timeline.started_at.is_none() {
            self.timeline.started_at = Some(now);
        }
        self.timeline.last_active_at = Some(now);
    }

    pub fn mark_active(&mut self) {
        self.timeline.last_active_at = Some(Utc::now());
    }

    pub fn increment_iteration(&mut self) {
        self.execution.iteration += 1;
        self.mark_active();
    }

    pub fn add_usage(&mut self, usage: &Usage) {
        self.tokens.add_usage(usage);
        self.mark_active();
    }

    pub fn add_model_duration(&mut self, duration_ms: u32) {
        self.latency.total_model_duration_ms += u64::from(duration_ms);
        self.mark_active();
    }

    pub fn add_tool_duration(&mut self, duration_ms: u32) {
        self.latency.total_tool_duration_ms += u64::from(duration_ms);
        self.mark_active();
    }

    pub fn add_tool_results(&mut self, success_count: usize, failure_count: usize) {
        self.tools.call_count += (success_count + failure_count) as u64;
        self.tools.success_count += success_count as u64;
        self.tools.failure_count += failure_count as u64;
        self.mark_active();
    }

    pub fn record_error(&mut self, error: impl Into<String>) {
        self.errors.count += 1;
        self.errors.last_error = Some(error.into());
        self.mark_active();
    }

    pub fn mark_finished(&mut self, reason: AgentTerminalReason) {
        let now = Utc::now();
        self.timeline.finished_at = Some(now);
        self.timeline.last_active_at = Some(now);
        self.execution.terminal_reason = Some(reason);
        if let Some(started_at) = self.timeline.started_at {
            self.timeline.total_duration_ms =
                Some((now - started_at).num_milliseconds().max(0) as u64);
        }
    }
}

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

    // === 执行上下文 ===
    /// 分层上下文（对话、记忆、人格等）
    #[serde(default)]
    pub context: Context,

    // === 观测指标 ===
    #[serde(default)]
    pub metrics: Metrics,
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
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}
