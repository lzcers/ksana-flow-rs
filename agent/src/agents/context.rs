//! 通用上下文 - 分层、类型化、可演化的数据容器
//!
//! Context 是 Agent 状态的核心部分，包含：
//! - 系统指令（System）
//! - 人格定义（Soul）
//! - 用户画像（User）
//! - 记忆（Memory）
//! - 对话历史（Conversation）
//! - 自定义层

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::core::Message;

/// 通用上下文 - 分层、类型化、可演化的数据容器
///
/// Context 是 Agent 状态的核心部分，包含：
/// - 系统指令（System）
/// - 人格定义（Soul）
/// - 用户画像（User）
/// - 记忆（Memory）
/// - 对话历史（Conversation）
/// - 自定义层
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    /// 层级数据
    pub layers: Vec<Layer>,
}

/// 数据层 - 可独立加载、卸载、序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// 层名称
    pub name: String,
    /// 层类型（决定如何解释和使用数据）
    pub kind: LayerKind,
    /// 数据内容
    pub data: Value,
    /// 元数据
    #[serde(default)]
    pub meta: LayerMeta,
}

/// 层类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerKind {
    /// 系统指令
    System,
    /// 人格/角色
    Soul,
    /// 用户画像
    User,
    /// 记忆（长期）
    Memory,
    /// 对话历史（短期）
    Conversation,
    /// 工具定义
    Tools,
    /// 自定义
    Custom(String),
}

/// 层元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerMeta {
    /// 来源（文件路径、URL 等）
    #[serde(default)]
    pub source: Option<String>,
    /// 优先级（高优先级在前）
    #[serde(default)]
    pub priority: i32,
    /// 是否只读
    #[serde(default)]
    pub readonly: bool,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Context {
    /// 创建空上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加层
    pub fn layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    /// 按名称获取层
    pub fn get(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.name == name)
    }

    /// 按名称获取可变层
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }

    /// 按类型获取所有层
    pub fn get_by_kind(&self, kind: &LayerKind) -> Vec<&Layer> {
        self.layers.iter().filter(|l| &l.kind == kind).collect()
    }

    /// 合并另一上下文
    pub fn merge(&mut self, other: Context) {
        for layer in other.layers {
            if let Some(existing) = self.layers.iter_mut().find(|l| l.name == layer.name) {
                // 合并数据
                if let (Value::Object(a), Value::Object(b)) = (&mut existing.data, &layer.data) {
                    for (k, v) in b {
                        a.insert(k.clone(), v.clone());
                    }
                } else {
                    existing.data = layer.data;
                }
            } else {
                self.layers.push(layer);
            }
        }
    }

    /// 按优先级排序并转换为消息列表
    pub fn to_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        // 按 priority 排序（高优先级在前）
        let mut sorted_layers: Vec<_> = self.layers.iter().collect();
        sorted_layers.sort_by(|a, b| b.meta.priority.cmp(&a.meta.priority));

        // 构建系统消息
        let system_parts: Vec<String> = sorted_layers
            .iter()
            .filter(|l| {
                matches!(
                    l.kind,
                    LayerKind::System | LayerKind::Soul | LayerKind::User | LayerKind::Memory
                )
            })
            .filter_map(|l| layer_to_system_content(l))
            .collect();

        if !system_parts.is_empty() {
            messages.push(Message::system(&system_parts.join("\n\n---\n\n")));
        }

        // 添加对话历史
        for layer in &sorted_layers {
            if layer.kind == LayerKind::Conversation
                && let Value::Array(arr) = &layer.data
            {
                for item in arr {
                    if let Ok(msg) = serde_json::from_value::<Message>(item.clone()) {
                        messages.push(msg);
                    }
                }
            }
        }

        messages
    }

    /// 获取对话历史
    pub fn conversation(&self) -> Vec<Message> {
        self.get_by_kind(&LayerKind::Conversation)
            .first()
            .and_then(|l| {
                if let Value::Array(arr) = &l.data {
                    Some(
                        arr.iter()
                            .filter_map(|item| serde_json::from_value::<Message>(item.clone()).ok())
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// 添加消息到对话历史
    pub fn add_message(&mut self, message: Message) {
        // 查找或创建对话层
        let conversation_layer = self
            .layers
            .iter_mut()
            .find(|l| l.kind == LayerKind::Conversation);

        if let Some(layer) = conversation_layer {
            if let Value::Array(ref mut arr) = layer.data {
                arr.push(serde_json::to_value(&message).unwrap_or(Value::Null));
            }
        } else {
            // 创建新的对话层
            self.layers.push(Layer {
                name: "conversation".to_string(),
                kind: LayerKind::Conversation,
                data: serde_json::to_value(vec![&message]).unwrap_or(Value::Null),
                meta: LayerMeta::default(),
            });
        }
    }

    /// 清空对话历史
    pub fn clear_conversation(&mut self) {
        if let Some(layer) = self
            .layers
            .iter_mut()
            .find(|l| l.kind == LayerKind::Conversation)
        {
            layer.data = Value::Array(Vec::new());
        }
    }
}

/// 将层转换为系统消息内容
fn layer_to_system_content(layer: &Layer) -> Option<String> {
    match &layer.kind {
        LayerKind::System => {
            if let Value::String(s) = &layer.data {
                Some(s.clone())
            } else {
                Some(layer.data.to_string())
            }
        }
        LayerKind::Soul => {
            let content = if let Value::Object(map) = &layer.data {
                let mut parts = Vec::new();
                if let Some(Some(name)) = map.get("name").map(|v| v.as_str()) {
                    parts.push(format!("# {}\n", name));
                }
                if let Some(Some(role)) = map.get("role").map(|v| v.as_str()) {
                    parts.push(format!("角色：{}\n", role));
                }
                if let Some(Value::Array(guidelines)) = map.get("guidelines") {
                    parts.push("准则：".to_string());
                    for g in guidelines {
                        if let Some(s) = g.as_str() {
                            parts.push(format!("- {}\n", s));
                        }
                    }
                }
                parts.join("\n")
            } else {
                layer.data.to_string()
            };
            Some(content)
        }
        LayerKind::User => {
            let content = if let Value::Object(map) = &layer.data {
                let mut parts = Vec::new();
                if let Some(Some(name)) = map.get("name").map(|v| v.as_str()) {
                    parts.push(format!("用户名：{}", name));
                }
                parts.join("\n")
            } else {
                layer.data.to_string()
            };
            Some(format!("# 用户信息\n\n{}", content))
        }
        LayerKind::Memory => {
            if let Value::Array(items) = &layer.data {
                let entries: Vec<String> = items
                    .iter()
                    .filter_map(|item| {
                        if let Value::Object(map) = item {
                            map.get("content")
                                .and_then(|v| v.as_str())
                                .map(|s| format!("- {}", s))
                        } else {
                            None
                        }
                    })
                    .collect();
                if entries.is_empty() {
                    None
                } else {
                    Some(format!("# 记忆\n\n{}", entries.join("\n")))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

impl Layer {
    /// 创建新层
    pub fn new(name: impl Into<String>, kind: LayerKind, data: Value) -> Self {
        Self {
            name: name.into(),
            kind,
            data,
            meta: LayerMeta::default(),
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.meta.priority = priority;
        self
    }

    /// 设置来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.meta.source = Some(source.into());
        self
    }

    /// 设置只读
    pub fn with_readonly(mut self, readonly: bool) -> Self {
        self.meta.readonly = readonly;
        self
    }
}

// ============================================================================
// ContextHandle - 共享 Context 的句柄
// ============================================================================

/// Context 的共享句柄，支持并发读取
///
/// 用于多 Agent 共享同一 Context 的黑板模式：
/// - Actor 通过 `read()` 获取只读快照
/// - Hook 通过 `write()` 更新 Context
/// - 多个 Agent 可以共享同一个 ContextHandle
#[derive(Debug, Clone)]
pub struct ContextHandle {
    inner: Arc<RwLock<Context>>,
}

impl ContextHandle {
    /// 创建新的 ContextHandle
    pub fn new(context: Context) -> Self {
        Self {
            inner: Arc::new(RwLock::new(context)),
        }
    }

    /// 读取当前 Context（快照）
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, Context> {
        self.inner.read().await
    }

    /// 写入 Context（由 Hook 调用）
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, Context> {
        self.inner.write().await
    }

    /// 获取内部 Arc 的克隆（用于直接传递给需要 Arc<RwLock<Context>> 的地方）
    pub fn inner(&self) -> Arc<RwLock<Context>> {
        self.inner.clone()
    }
}

impl Default for ContextHandle {
    fn default() -> Self {
        Self::new(Context::new())
    }
}

impl From<Context> for ContextHandle {
    fn from(context: Context) -> Self {
        Self::new(context)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_layer() {
        let ctx = Context::new()
            .layer(
                Layer::new(
                    "system",
                    LayerKind::System,
                    Value::String("You are helpful.".into()),
                )
                .with_priority(100),
            )
            .layer(
                Layer::new(
                    "soul",
                    LayerKind::Soul,
                    serde_json::json!({
                        "name": "Kśana",
                        "role": "AI Assistant",
                        "guidelines": ["Be helpful", "Be concise"]
                    }),
                )
                .with_priority(50),
            );

        assert_eq!(ctx.layers.len(), 2);

        let system = ctx.get("system").unwrap();
        assert_eq!(system.meta.priority, 100);
    }

    #[test]
    fn test_context_to_messages() {
        let ctx = Context::new()
            .layer(Layer::new(
                "system",
                LayerKind::System,
                Value::String("Be helpful.".into()),
            ))
            .layer(Layer::new(
                "conversation",
                LayerKind::Conversation,
                serde_json::json!([
                    {"role": "user", "content": "Hello"},
                    {"role": "assistant", "content": "Hi!"}
                ]),
            ));

        let messages = ctx.to_messages();
        assert_eq!(messages.len(), 3); // system + 2 conversation
        assert!(matches!(messages[0], Message::System { .. }));
    }

    #[test]
    fn test_context_add_message() {
        let mut ctx = Context::new();
        ctx.add_message(Message::user("Hello"));
        ctx.add_message(Message::assistant("Hi!"));

        let conv = ctx.conversation();
        assert_eq!(conv.len(), 2);
    }
}
