//! Hook 机制 - 处理 Agent 执行过程中的副作用
//!
//! Hook 系统允许在 Agent 执行过程中插入自定义逻辑：
//! - 日志记录
//! - Context 更新
//! - 指标收集
//! - 持久化
//!
//! Hook 按优先级顺序执行，支持失败处理策略。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::agent_actor::AgentActorEvent;
use super::context::ContextHandle;
use crate::core::Message;

// ============================================================================
// Hook 配置
// ============================================================================

/// Hook 配置
#[derive(Debug, Clone)]
pub struct HookConfig {
    /// 优先级（数值越小越先执行）
    pub priority: u32,
    /// 失败时是否继续执行后续 Hook
    pub fail_continue: bool,
    /// 是否启用
    pub enabled: bool,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            priority: 100,
            fail_continue: true,
            enabled: true,
        }
    }
}

// ============================================================================
// Hook 错误类型
// ============================================================================

/// Hook 执行错误
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// Context 更新错误
    #[error("Context update error: {0}")]
    Context(String),
    /// IO 错误
    #[error("IO error: {0}")]
    Io(String),
    /// 自定义错误
    #[error("Custom error: {0}")]
    Custom(String),
}

// ============================================================================
// AgentEvent 扩展
// ============================================================================

/// Agent 事件扩展 - 包含建议性的 Context 更新操作
///
/// 这些事件用于 Actor 向 Hook 系统建议更新操作，
/// 由 Hook 决定是否实际应用这些更新。
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// 代理 AgentActorEvent（现有事件透传）
    ActorEvent(AgentActorEvent),

    // === Context 变化建议 ===
    /// Actor 建议添加消息（由 Hook 决定是否应用）
    SuggestAddMessage {
        /// 要添加的消息
        message: Message,
    },
    /// Actor 建议更新层（由 Hook 决定是否应用）
    SuggestUpdateLayer {
        /// 层名称
        name: String,
        /// 新的数据
        data: Value,
    },
}

impl From<AgentActorEvent> for AgentEvent {
    fn from(event: AgentActorEvent) -> Self {
        AgentEvent::ActorEvent(event)
    }
}

// ============================================================================
// AgentHook Trait
// ============================================================================

/// Hook trait - 处理 Agent 执行过程中的副作用
///
/// 实现此 trait 以创建自定义 Hook：
/// - `on_event`: 处理事件的核心逻辑
/// - `name`: Hook 名称（用于日志和调试）
/// - `config`: Hook 配置（优先级、失败策略等）
#[async_trait]
pub trait AgentHook: Send + Sync {
    /// Hook 名称
    fn name(&self) -> &str;

    /// 处理事件
    async fn on_event(&self, event: &AgentEvent) -> Result<(), HookError>;

    /// 获取配置（子类应重写此方法）
    fn config(&self) -> HookConfig {
        HookConfig::default()
    }
}

// ============================================================================
// Hook 注册表
// ============================================================================

/// Hook 容器 - 管理多个 Hook 的执行
pub struct HookRegistry {
    hooks: Vec<Arc<dyn AgentHook>>,
}

impl HookRegistry {
    /// 创建空的 Hook 注册表
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 注册 Hook
    pub fn register(&mut self, hook: Arc<dyn AgentHook>) {
        self.hooks.push(hook);
        // 按优先级排序（数值小的在前）
        self.hooks.sort_by_key(|h| h.config().priority);
    }

    /// 移除指定名称的 Hook
    pub fn remove(&mut self, name: &str) -> bool {
        let len_before = self.hooks.len();
        self.hooks.retain(|h| h.name() != name);
        self.hooks.len() != len_before
    }

    /// 获取所有 Hook 的名称
    pub fn hook_names(&self) -> Vec<&str> {
        self.hooks.iter().map(|h| h.name()).collect()
    }

    /// 顺序执行所有 Hook
    pub async fn emit(&self, event: &AgentEvent) -> Result<(), HookError> {
        for hook in &self.hooks {
            let config = hook.config();
            if !config.enabled {
                continue;
            }
            match hook.on_event(event).await {
                Ok(()) => {}
                Err(e) => {
                    if !config.fail_continue {
                        return Err(e);
                    }
                    // 记录错误但继续执行
                    eprintln!("[Hook {}] Error: {}", hook.name(), e);
                }
            }
        }
        Ok(())
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// 获取 Hook 数量
    pub fn len(&self) -> usize {
        self.hooks.len()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置 Hook 实现
// ============================================================================

/// Context 更新 Hook
///
/// 处理 `SuggestAddMessage` 和 `SuggestUpdateLayer` 事件，
/// 将建议性的更新应用到实际的 Context 中。
///
/// 优先级默认为 1000（最后执行），确保其他 Hook 先处理事件。
pub struct ContextUpdateHook {
    name: String,
    config: HookConfig,
    context: ContextHandle,
}

impl ContextUpdateHook {
    /// 创建新的 Context 更新 Hook
    pub fn new(context: ContextHandle) -> Self {
        Self {
            name: "context_update".to_string(),
            config: HookConfig {
                priority: 1000,       // 最后执行
                fail_continue: false, // Context 更新失败应停止
                enabled: true,
            },
            context,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.config.priority = priority;
        self
    }
}

#[async_trait]
impl AgentHook for ContextUpdateHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> HookConfig {
        self.config.clone()
    }

    async fn on_event(&self, event: &AgentEvent) -> Result<(), HookError> {
        match event {
            AgentEvent::SuggestAddMessage { message } => {
                self.context.write().await.add_message(message.clone());
            }
            AgentEvent::SuggestUpdateLayer { name, data } => {
                if let Some(layer) = self.context.write().await.get_mut(name) {
                    layer.data = data.clone();
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// 日志 Hook
///
/// 打印所有事件到标准输出，用于调试和监控。
///
/// 优先级默认为 0（最先执行），确保日志先于其他处理。
pub struct LoggingHook {
    name: String,
    config: HookConfig,
    prefix: String,
}

impl LoggingHook {
    /// 创建新的日志 Hook
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            name: "logging".to_string(),
            config: HookConfig {
                priority: 0,         // 最先执行
                fail_continue: true, // 日志失败不影响其他 Hook
                enabled: true,
            },
            prefix: prefix.into(),
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.config.priority = priority;
        self
    }

    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }
}

#[async_trait]
impl AgentHook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> HookConfig {
        self.config.clone()
    }

    async fn on_event(&self, event: &AgentEvent) -> Result<(), HookError> {
        println!("[{}] {:?}", self.prefix, event);
        Ok(())
    }
}
