//! Reducer 模式实现
//!
//! 将 Agent 从有状态结构体重构为无状态的 reducer 函数设计。
//! 核心思想：分离"决策"与"执行"
//!
//! - Reducer: 纯函数，状态 + 输入 -> (新状态, 输出指令)
//! - Runner: 执行器，根据指令执行副作用

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::agents::{ToolCall, ToolDef, ToolExecutorError, ToolResult};
use crate::core::Message;

// ============================================================================
// Context: 分层上下文
// ============================================================================

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
            .filter(|l| matches!(l.kind, LayerKind::System | LayerKind::Soul | LayerKind::User | LayerKind::Memory))
            .filter_map(|l| layer_to_system_content(l))
            .collect();

        if !system_parts.is_empty() {
            messages.push(Message::system(&system_parts.join("\n\n---\n\n")));
        }

        // 添加对话历史
        for layer in &sorted_layers {
            if layer.kind == LayerKind::Conversation && let Value::Array(arr) = &layer.data {
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
        if let Some(layer) = self.layers.iter_mut().find(|l| l.kind == LayerKind::Conversation) {
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
                            map.get("content").and_then(|v| v.as_str()).map(|s| format!("- {}", s))
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
// AgentState: Agent 状态
// ============================================================================

/// Agent 状态 - 可持久化、可追踪的执行实体
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// 状态转换历史
    #[serde(default)]
    pub transitions: Vec<StateTransition>,

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
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_max_iterations() -> usize {
    10
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new("")
    }
}

/// 作业状态
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

/// 状态转换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// 源状态
    pub from: JobState,
    /// 目标状态
    pub to: JobState,
    /// 转换时间
    pub timestamp: DateTime<Utc>,
    /// 转换原因
    pub reason: Option<String>,
    /// 触发者
    pub triggered_by: Option<String>,
}

impl AgentState {
    /// 创建新状态
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            job_id: Uuid::new_v4(),
            user_id: user_id.into(),
            conversation_id: None,
            title: String::new(),
            description: String::new(),
            category: None,
            state: JobState::Pending,
            transitions: Vec::new(),
            budget: None,
            actual_cost: Decimal::ZERO,
            context: Context::new(),
            iteration: 0,
            max_iterations: 10,
        }
    }

    /// 设置标题
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// 设置预算
    pub fn with_budget(mut self, budget: Decimal) -> Self {
        self.budget = Some(budget);
        self
    }

    /// 设置最大迭代次数
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// 设置上下文
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    /// 添加用户消息（快捷方式）
    pub fn with_user_message(mut self, content: &str) -> Self {
        self.context.add_message(Message::user(content));
        self
    }

    /// 状态转换
    pub fn transition(&mut self, to: JobState, reason: Option<String>) {
        let from = self.state;
        self.transitions.push(StateTransition {
            from,
            to,
            timestamp: Utc::now(),
            reason,
            triggered_by: None,
        });
        self.state = to;
    }

    /// 是否可执行
    pub fn is_runnable(&self) -> bool {
        matches!(
            self.state,
            JobState::Pending | JobState::Running | JobState::WaitingInput
        )
    }

    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled
        )
    }

    /// 检查预算
    pub fn check_budget(&self) -> bool {
        match self.budget {
            Some(budget) => self.actual_cost <= budget,
            None => true,
        }
    }

    /// 添加成本
    pub fn add_cost(&mut self, cost: Decimal) {
        self.actual_cost += cost;
    }

    /// 获取对话历史
    pub fn messages(&self) -> Vec<Message> {
        self.context.to_messages()
    }

    /// 获取最后一条消息
    pub fn last_message(&self) -> Option<Message> {
        self.context.conversation().last().cloned()
    }

    /// 添加消息
    pub fn add_message(&mut self, message: Message) {
        self.context.add_message(message);
    }
}

// ============================================================================
// Reducer 类型
// ============================================================================

/// Agent 输入
#[derive(Debug, Clone)]
pub enum AgentInput {
    /// 用户消息
    UserMessage(Message),
    /// 继续执行
    Continue,
}

/// Agent 输出（reducer 返回值）
#[derive(Debug, Clone)]
pub enum AgentOutput {
    /// 需要调用模型
    ChatRequest {
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
    },
    /// 需要执行工具
    ToolCalls(Vec<ToolCall>),
    /// 执行完成
    Complete,
    /// 达到最大迭代
    MaxIterationsReached,
    /// 预算超限
    BudgetExceeded,
}

/// Agent 副作用结果
#[derive(Debug, Clone)]
pub enum AgentEffectResult {
    /// LLM 响应
    ChatResponse(Message),
    /// 工具执行结果
    ToolResults(Vec<(ToolCall, Result<ToolResult, ToolExecutorError>)>),
}

/// Agent 配置
#[derive(Debug, Clone, Default)]
pub struct AgentConfig {
    /// 可用工具
    pub tools: Vec<ToolDef>,
    /// 最大迭代次数
    pub max_iterations: usize,
}

impl AgentConfig {
    /// 创建新配置
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            max_iterations: 10,
        }
    }

    /// 添加工具
    pub fn with_tool(mut self, tool: ToolDef) -> Self {
        self.tools.push(tool);
        self
    }

    /// 设置工具列表
    pub fn with_tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.tools = tools;
        self
    }

    /// 设置最大迭代次数
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
}

// ============================================================================
// Reducer 函数
// ============================================================================

/// 纯函数：状态 + 输入 -> (新状态, 输出)
///
/// 这个函数不执行任何 IO，只做状态转换
pub fn reduce(
    mut state: AgentState,
    input: AgentInput,
    config: &AgentConfig,
) -> (AgentState, AgentOutput) {
    match input {
        AgentInput::UserMessage(msg) => {
            // 添加用户消息到上下文
            state.context.add_message(msg);

            // 检查预算
            if !state.check_budget() {
                state.transition(JobState::Failed, Some("预算超限".to_string()));
                return (state, AgentOutput::BudgetExceeded);
            }

            // 请求调用模型
            let messages = state.messages();
            (
                state,
                AgentOutput::ChatRequest {
                    messages,
                    tools: config.tools.clone(),
                },
            )
        }

        AgentInput::Continue => {
            // 检查状态
            if state.is_finished() {
                return (state, AgentOutput::Complete);
            }

            // 检查迭代次数
            if state.iteration >= config.max_iterations {
                state.transition(
                    JobState::Failed,
                    Some(format!("达到最大迭代次数: {}", config.max_iterations)),
                );
                return (state, AgentOutput::MaxIterationsReached);
            }

            // 检查预算
            if !state.check_budget() {
                state.transition(JobState::Failed, Some("预算超限".to_string()));
                return (state, AgentOutput::BudgetExceeded);
            }

            // 根据最后一条消息决定下一步
            match state.last_message() {
                Some(Message::Assistant {
                    tool_calls: Some(calls),
                    ..
                }) if !calls.is_empty() => {
                    // 需要执行工具
                    (state, AgentOutput::ToolCalls(calls))
                }
                Some(Message::Assistant { .. }) => {
                    // Assistant 已回复且无工具调用，完成
                    state.transition(JobState::Completed, Some("任务完成".to_string()));
                    (state, AgentOutput::Complete)
                }
                _ => {
                    // 需要调用模型
                    let messages = state.messages();
                    (
                        state,
                        AgentOutput::ChatRequest {
                            messages,
                            tools: config.tools.clone(),
                        },
                    )
                }
            }
        }
    }
}

/// 应用副作用结果到状态
pub fn apply_effect(
    mut state: AgentState,
    effect: AgentEffectResult,
    config: &AgentConfig,
) -> (AgentState, AgentOutput) {
    match effect {
        AgentEffectResult::ChatResponse(response) => {
            // 添加助手回复到上下文
            state.context.add_message(response);
            // 增加迭代计数
            state.iteration += 1;
            // 继续处理
            reduce(state, AgentInput::Continue, config)
        }

        AgentEffectResult::ToolResults(results) => {
            // 添加工具结果到上下文
            for (call, result) in results {
                let tool_msg = match result {
                    Ok(r) => Message::Tool {
                        tool_call_id: r.id,
                        content: serde_json::to_string(&r.output).unwrap_or_default(),
                    },
                    Err(e) => Message::Tool {
                        tool_call_id: call.id,
                        content: format!("Error: {}", e),
                    },
                };
                state.context.add_message(tool_msg);
            }
            // 继续处理
            reduce(state, AgentInput::Continue, config)
        }
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
                Layer::new("system", LayerKind::System, Value::String("You are helpful.".into()))
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
            .layer(Layer::new("system", LayerKind::System, Value::String("Be helpful.".into())))
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
    fn test_agent_state_new() {
        let state = AgentState::new("user-123")
            .with_title("Test Job")
            .with_budget(Decimal::from(1));

        assert_eq!(state.user_id, "user-123");
        assert_eq!(state.title, "Test Job");
        assert!(state.budget.is_some());
        assert_eq!(state.state, JobState::Pending);
    }

    #[test]
    fn test_agent_state_transition() {
        let mut state = AgentState::new("user-123");
        assert_eq!(state.state, JobState::Pending);

        state.transition(JobState::Running, Some("开始执行".into()));
        assert_eq!(state.state, JobState::Running);
        assert_eq!(state.transitions.len(), 1);
        assert_eq!(state.transitions[0].from, JobState::Pending);
        assert_eq!(state.transitions[0].to, JobState::Running);
    }

    #[test]
    fn test_reduce_user_message() {
        let state = AgentState::new("user-123");
        let config = AgentConfig::new();

        let (new_state, output) = reduce(state, AgentInput::UserMessage(Message::user("Hello")), &config);

        assert_eq!(new_state.context.conversation().len(), 1);
        match output {
            AgentOutput::ChatRequest { messages, .. } => {
                assert!(messages.iter().any(|m| m.content().contains("Hello")));
            }
            _ => panic!("Expected ChatRequest"),
        }
    }

    #[test]
    fn test_reduce_max_iterations() {
        let mut state = AgentState::new("user-123");
        state.iteration = 10;
        let config = AgentConfig::new().with_max_iterations(10);

        let (new_state, output) = reduce(state, AgentInput::Continue, &config);

        assert!(matches!(output, AgentOutput::MaxIterationsReached));
        assert_eq!(new_state.state, JobState::Failed);
    }

    #[test]
    fn test_reduce_complete() {
        let mut state = AgentState::new("user-123");
        state.context.add_message(Message::user("Hello"));
        state.context.add_message(Message::assistant("Hi there!"));

        let config = AgentConfig::new();

        let (new_state, output) = reduce(state, AgentInput::Continue, &config);

        assert!(matches!(output, AgentOutput::Complete));
        assert_eq!(new_state.state, JobState::Completed);
    }

    #[test]
    fn test_reduce_tool_calls() {
        let mut state = AgentState::new("user-123");
        state.context.add_message(Message::user("Search for weather"));
        state.context.add_message(Message::Assistant {
            content: String::new(),
            tool_calls: Some(vec![ToolCall {
                id: "call-1".into(),
                call_type: Some("function".into()),
                index: None,
                function: Some(crate::agents::ToolCallFunction {
                    name: "search".into(),
                    arguments: r#"{"query": "weather"}"#.into(),
                }),
                name: None,
                arguments: None,
            }]),
        });

        let config = AgentConfig::new();

        let (_new_state, output) = reduce(state, AgentInput::Continue, &config);

        match output {
            AgentOutput::ToolCalls(calls) => {
                assert_eq!(calls.len(), 1);
            }
            _ => panic!("Expected ToolCalls"),
        }
    }

    #[test]
    fn test_apply_effect_chat_response() {
        let mut state = AgentState::new("user-123");
        state.context.add_message(Message::user("Hello"));
        let config = AgentConfig::new();

        let response = Message::assistant("Hi there!");
        let (new_state, output) = apply_effect(state, AgentEffectResult::ChatResponse(response), &config);

        assert_eq!(new_state.iteration, 1);
        assert!(matches!(output, AgentOutput::Complete));
    }

    #[test]
    fn test_apply_effect_tool_results() {
        let mut state = AgentState::new("user-123");
        state.iteration = 1;
        let config = AgentConfig::new();

        let call = ToolCall {
            id: "call-1".into(),
            call_type: Some("function".into()),
            index: None,
            function: Some(crate::agents::ToolCallFunction {
                name: "search".into(),
                arguments: r#"{"query": "weather"}"#.into(),
            }),
            name: None,
            arguments: None,
        };

        let result = ToolResult {
            id: "call-1".into(),
            success: true,
            output: serde_json::json!({"result": "sunny"}),
        };

        let (_new_state, _output) = apply_effect(
            state,
            AgentEffectResult::ToolResults(vec![(call.clone(), Ok(result))]),
            &config,
        );

        // 验证工具结果被添加到上下文
    }

    #[test]
    fn test_context_add_message() {
        let mut ctx = Context::new();
        ctx.add_message(Message::user("Hello"));
        ctx.add_message(Message::assistant("Hi!"));

        let conv = ctx.conversation();
        assert_eq!(conv.len(), 2);
    }

    #[test]
    fn test_agent_state_with_user_message() {
        let state = AgentState::new("user-123")
            .with_user_message("Hello");

        assert_eq!(state.context.conversation().len(), 1);
    }
}