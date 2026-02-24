//! Memory 模块 - Agent 记忆存储
//!
//! 设计原则：
//! - 核心 Memory trait 最小化
//! - 扩展能力通过独立 trait 提供
//! - 序列化是实现细节，不在核心 trait 中

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::Message;

// ============================================================================
// 核心类型
// ============================================================================

/// 带元数据的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memo {
    /// 唯一标识
    pub id: String,
    /// 消息内容
    pub message: Message,
    /// 创建时间
    pub timestamp: DateTime<Utc>,
    /// 重要性分数 (0.0 - 1.0)，用于摘要优先级
    #[serde(default)]
    pub importance: f32,
}

impl Memo {
    pub fn new(id: impl Into<String>, message: Message) -> Self {
        Self {
            id: id.into(),
            message,
            timestamp: Utc::now(),
            importance: 0.5,
        }
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// 从 Message 创建，自动生成 ID
    pub fn from_message(message: Message) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self::new(id, message)
    }
}

/// Memory 操作错误
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// ============================================================================
// 核心 Trait: Memory
// ============================================================================

/// Memory 核心能力 - 存储和检索消息
///
/// 这是最小化的核心接口，所有 Memory 实现都必须提供：
/// - 添加消息
/// - 获取所有消息
/// - 清空记忆
#[async_trait]
pub trait Memory: Send + Sync {
    /// 添加一条消息到记忆
    async fn add(&self, message: &Message);

    /// 批量添加消息
    async fn add_batch(&self, messages: &[Message]) {
        for msg in messages {
            self.add(msg).await;
        }
    }

    /// 获取所有消息（按时间顺序）
    async fn get_all(&self) -> Vec<Message>;

    /// 获取所有带元数据的消息
    async fn get_memos(&self) -> Vec<Memo> {
        // 默认实现：从 get_all 转换
        self.get_all()
            .await
            .into_iter()
            .map(Memo::from_message)
            .collect()
    }

    /// 获取消息数量
    async fn len(&self) -> usize {
        self.get_all().await.len()
    }

    /// 判断是否为空
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 清空所有记忆
    async fn clear(&self);
}

// ============================================================================
// 扩展 Trait: PersistentMemory
// ============================================================================

/// 持久化能力 - 支持保存和加载
///
/// 实现此 trait 的 Memory 可以将状态持久化到文件、数据库等
#[async_trait]
pub trait PersistentMemory: Memory {
    /// 序列化为字符串
    async fn serialize(&self) -> Result<String, MemoryError>;

    /// 从字符串反序列化
    async fn deserialize(&self, data: &str) -> Result<(), MemoryError>;

    /// 保存到文件
    async fn save_to_file(&self, path: &std::path::Path) -> Result<(), MemoryError> {
        let data = self.serialize().await?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    /// 从文件加载
    async fn load_from_file(&self, path: &std::path::Path) -> Result<(), MemoryError> {
        let data = tokio::fs::read_to_string(path).await?;
        self.deserialize(&data).await
    }
}

// ============================================================================
// 扩展 Trait: SemanticMemory
// ============================================================================

/// 语义检索能力 - 支持 RAG
///
/// 实现此 trait 的 Memory 可以根据语义相关性检索消息
#[async_trait]
pub trait SemanticMemory: Memory {
    /// 根据查询获取相关消息
    ///
    /// # 参数
    /// - `query`: 查询文本
    /// - `limit`: 返回的最大消息数
    ///
    /// # 返回
    /// 按相关性排序的消息列表
    async fn get_relevant(&self, query: &str, limit: usize) -> Vec<Message>;

    /// 获取相关消息（带分数）
    async fn get_relevant_with_scores(&self, query: &str, limit: usize) -> Vec<(Message, f32)> {
        // 默认实现：返回 get_relevant 的结果，分数为 1.0
        self.get_relevant(query, limit)
            .await
            .into_iter()
            .map(|m| (m, 1.0))
            .collect()
    }
}

// ============================================================================
// 实现: SlidingWindowMemory
// ============================================================================

/// 滑动窗口记忆 - 保留最近 N 条消息
pub struct SlidingWindowMemory {
    memos: Arc<Mutex<Vec<Memo>>>,
    max_messages: usize,
}

impl SlidingWindowMemory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            memos: Arc::new(Mutex::new(Vec::new())),
            max_messages,
        }
    }

    /// 创建无限制的记忆
    pub fn unbounded() -> Self {
        Self {
            memos: Arc::new(Mutex::new(Vec::new())),
            max_messages: usize::MAX,
        }
    }
}

#[async_trait]
impl Memory for SlidingWindowMemory {
    async fn add(&self, message: &Message) {
        let mut memos = self.memos.lock().await;

        if memos.len() >= self.max_messages {
            memos.remove(0);
        }

        memos.push(Memo::from_message(message.clone()));
    }

    async fn get_all(&self) -> Vec<Message> {
        self.memos
            .lock()
            .await
            .iter()
            .map(|m| m.message.clone())
            .collect()
    }

    async fn get_memos(&self) -> Vec<Memo> {
        self.memos.lock().await.clone()
    }

    async fn clear(&self) {
        self.memos.lock().await.clear();
    }
}

// ============================================================================
// 实现: MarkdownMemory (持久化)
// ============================================================================

/// Markdown 格式的持久化记忆
///
/// 使用 Markdown 文件存储对话历史，适合：
/// - 调试和查看
/// - 版本控制
/// - 人工编辑
pub struct MarkdownMemory {
    memos: Arc<Mutex<Vec<Memo>>>,
    max_messages: usize,
}

impl MarkdownMemory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            memos: Arc::new(Mutex::new(Vec::new())),
            max_messages,
        }
    }

    pub fn unbounded() -> Self {
        Self {
            memos: Arc::new(Mutex::new(Vec::new())),
            max_messages: usize::MAX,
        }
    }
}

#[async_trait]
impl Memory for MarkdownMemory {
    async fn add(&self, message: &Message) {
        let mut memos = self.memos.lock().await;

        if memos.len() >= self.max_messages {
            memos.remove(0);
        }

        memos.push(Memo::from_message(message.clone()));
    }

    async fn get_all(&self) -> Vec<Message> {
        self.memos
            .lock()
            .await
            .iter()
            .map(|m| m.message.clone())
            .collect()
    }

    async fn get_memos(&self) -> Vec<Memo> {
        self.memos.lock().await.clone()
    }

    async fn clear(&self) {
        self.memos.lock().await.clear();
    }
}

#[async_trait]
impl PersistentMemory for MarkdownMemory {
    async fn serialize(&self) -> Result<String, MemoryError> {
        let memos = self.memos.lock().await;
        Ok(memos_to_markdown(&memos))
    }

    async fn deserialize(&self, markdown: &str) -> Result<(), MemoryError> {
        let memos = parse_markdown_to_memos(markdown)?;
        let mut stored = self.memos.lock().await;
        *stored = memos;
        Ok(())
    }
}

// ============================================================================
// 实现: ContextualMemory
// ============================================================================

/// 组合记忆 - 系统提示 + 滑动窗口
///
/// 常见模式：系统提示始终保留，对话历史使用滑动窗口
pub struct ContextualMemory {
    system_prompt: Arc<Mutex<Option<Memo>>>,
    conversation: SlidingWindowMemory,
}

impl ContextualMemory {
    pub fn new(max_conversation_messages: usize) -> Self {
        Self {
            system_prompt: Arc::new(Mutex::new(None)),
            conversation: SlidingWindowMemory::new(max_conversation_messages),
        }
    }

    pub fn with_system_prompt(self, prompt: String) -> Self {
        // 使用 try_lock 避免阻塞
        if let Ok(mut guard) = self.system_prompt.try_lock() {
            *guard = Some(Memo::new("system", Message::system(prompt)));
        }
        self
    }

    /// 设置系统提示
    pub async fn set_system_prompt(&self, prompt: String) {
        let mut system = self.system_prompt.lock().await;
        *system = Some(Memo::new("system", Message::system(prompt)));
    }

    /// 获取完整的消息列表（系统提示 + 对话历史）
    pub async fn get_full_context(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        if let Some(system) = self.system_prompt.lock().await.clone() {
            messages.push(system.message);
        }

        messages.extend(self.conversation.get_all().await);

        messages
    }
}

#[async_trait]
impl Memory for ContextualMemory {
    async fn add(&self, message: &Message) {
        if matches!(message, Message::System { .. }) {
            let mut system = self.system_prompt.lock().await;
            *system = Some(Memo::new("system", message.clone()));
        } else {
            self.conversation.add(message).await;
        }
    }

    async fn get_all(&self) -> Vec<Message> {
        self.get_full_context().await
    }

    async fn clear(&self) {
        *self.system_prompt.lock().await = None;
        self.conversation.clear().await;
    }
}

// ============================================================================
// Markdown 序列化/反序列化 (内部实现)
// ============================================================================

/// 将 Memo 列表转换为 Markdown
fn memos_to_markdown(memos: &[Memo]) -> String {
    let mut md = String::new();

    for memo in memos {
        md.push_str(&format!("<!-- id: {} -->\n", memo.id));
        md.push_str(&format!(
            "<!-- timestamp: {} -->\n",
            memo.timestamp.to_rfc3339()
        ));
        if memo.importance != 0.5 {
            md.push_str(&format!("<!-- importance: {} -->\n", memo.importance));
        }
        md.push_str(&message_to_markdown(&memo.message));
        md.push_str("\n---\n\n");
    }

    md
}

/// 将 Message 转换为 Markdown 格式
fn message_to_markdown(msg: &Message) -> String {
    match msg {
        Message::System { content } => {
            format!("## 🤖 System\n\n{}\n", content)
        }
        Message::User { content } => {
            format!("## 👤 User\n\n{}\n", content)
        }
        Message::Assistant {
            content,
            tool_calls,
        } => {
            let mut md = format!("## 🤖 Assistant\n\n{}\n", content);
            if let Some(calls) = tool_calls {
                if !calls.is_empty() {
                    md.push_str("\n### Tool Calls\n\n");
                    for call in calls {
                        md.push_str(&format!("- **{}** (`{}`)\n", call.get_name(), call.id));
                    }
                }
            }
            md
        }
        Message::Tool {
            tool_call_id,
            content,
        } => {
            format!("## 🔧 Tool Result (`{}`)\n\n{}\n", tool_call_id, content)
        }
    }
}

/// 从 Markdown 解析 Memo 列表
fn parse_markdown_to_memos(markdown: &str) -> Result<Vec<Memo>, MemoryError> {
    let mut memos = Vec::new();

    for section in markdown.split("\n---\n") {
        let section = section.trim();
        if section.is_empty() {
            continue;
        }

        // 解析 HTML 注释中的元数据
        let mut id = uuid::Uuid::new_v4().to_string();
        let mut timestamp = Utc::now();
        let mut importance = 0.5;

        for line in section.lines() {
            let line = line.trim();
            if line.starts_with("<!-- id:") {
                id = line
                    .trim_start_matches("<!-- id:")
                    .trim_end_matches("-->")
                    .trim()
                    .to_string();
            } else if line.starts_with("<!-- timestamp:") {
                let ts_str = line
                    .trim_start_matches("<!-- timestamp:")
                    .trim_end_matches("-->")
                    .trim();
                if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                    timestamp = ts.with_timezone(&Utc);
                }
            } else if line.starts_with("<!-- importance:") {
                let imp_str = line
                    .trim_start_matches("<!-- importance:")
                    .trim_end_matches("-->")
                    .trim();
                if let Ok(imp) = imp_str.parse::<f32>() {
                    importance = imp;
                }
            }
        }

        // 解析消息
        if let Some(message) = parse_message_from_section(section) {
            memos.push(
                Memo::new(id, message)
                    .with_importance(importance)
                    .with_timestamp(timestamp),
            );
        }
    }

    Ok(memos)
}

/// 从 Markdown section 解析 Message
fn parse_message_from_section(section: &str) -> Option<Message> {
    let lines: Vec<&str> = section.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // 找到标题行
    let header_idx = lines.iter().position(|l| l.starts_with("## "))?;
    let header = lines[header_idx].trim();

    // 提取内容（跳过标题和空行）
    let content_start = lines[header_idx..]
        .iter()
        .position(|l| l.trim().is_empty())
        .unwrap_or(1)
        + header_idx
        + 1;

    let content = if content_start < lines.len() {
        lines[content_start..]
            .iter()
            .take_while(|l| !l.starts_with("### ") && !l.starts_with("<!-- "))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    } else {
        String::new()
    };

    let msg = if header.contains("System") {
        Message::system(content)
    } else if header.contains("User") {
        Message::user(content)
    } else if header.contains("Assistant") {
        Message::assistant(content)
    } else if header.contains("Tool") {
        let id = extract_tool_call_id(header).unwrap_or_default();
        Message::Tool {
            tool_call_id: id,
            content,
        }
    } else {
        return None;
    };

    Some(msg)
}

/// 从标题提取 tool_call_id
fn extract_tool_call_id(header: &str) -> Option<String> {
    let start = header.find('`')?;
    let end = header[start + 1..].find('`')?;
    Some(header[start + 1..start + 1 + end].to_string())
}

// ============================================================================
// Memo 扩展方法
// ============================================================================

impl Memo {
    /// 设置时间戳
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sliding_window_memory() {
        let memory = SlidingWindowMemory::new(3);

        memory.add(&Message::user("Hello")).await;
        memory.add(&Message::assistant("Hi there!")).await;
        memory.add(&Message::user("How are you?")).await;

        assert_eq!(memory.len().await, 3);

        // 添加第 4 条，应该移除第 1 条
        memory.add(&Message::assistant("I'm good!")).await;
        assert_eq!(memory.len().await, 3);

        let messages = memory.get_all().await;
        assert!(messages[0].content().contains("Hi there"));
    }

    #[tokio::test]
    async fn test_markdown_memory_persistence() {
        let memory = MarkdownMemory::new(10);

        memory.add(&Message::system("You are helpful.")).await;
        memory.add(&Message::user("Hello")).await;
        memory.add(&Message::assistant("Hi!")).await;

        // 序列化
        let md = memory.serialize().await.unwrap();
        assert!(md.contains("## 🤖 System"));
        assert!(md.contains("## 👤 User"));
        assert!(md.contains("## 🤖 Assistant"));

        // 反序列化到新实例
        let memory2 = MarkdownMemory::new(10);
        memory2.deserialize(&md).await.unwrap();

        assert_eq!(memory2.len().await, 3);

        let messages = memory2.get_all().await;
        assert!(messages[0].content().contains("You are helpful"));
    }

    #[tokio::test]
    async fn test_contextual_memory() {
        let memory = ContextualMemory::new(5)
            .with_system_prompt("You are helpful.".to_string());

        memory.add(&Message::user("Hello")).await;
        memory.add(&Message::assistant("Hi!")).await;

        let messages = memory.get_all().await;

        assert_eq!(messages.len(), 3); // system + 2 messages
        assert!(matches!(messages[0], Message::System { .. }));
    }

    #[test]
    fn test_memo_metadata() {
        let memo = Memo::from_message(Message::user("Test"))
            .with_importance(0.9);

        assert_eq!(memo.importance, 0.9);
        assert!(!memo.id.is_empty());
    }

    #[tokio::test]
    async fn test_memo_preserved_in_memory() {
        let memory = SlidingWindowMemory::unbounded();
        memory.add(&Message::user("Test")).await;

        let memos = memory.get_memos().await;
        assert_eq!(memos.len(), 1);
        assert!(memos[0].timestamp.timestamp() > 0);
    }
}