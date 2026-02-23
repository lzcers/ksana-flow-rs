mod agent;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error; // 用于灵活的参数和结果

/// 工具定义，用于告知模型可用的工具。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// 工具名称，模型在调用时使用此名称。
    pub name: String,
    /// 工具描述，帮助模型理解何时使用该工具。
    pub description: String,
    /// 参数 JSON Schema，描述工具接受的参数格式。
    /// 通常是一个符合 JSON Schema 规范的对象。
    pub parameters: Value,
}

/// 模型发起的工具调用请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用的唯一标识符，用于将结果与调用关联。
    /// 通常由模型生成，执行结果需原样返回。
    pub id: String,
    /// 要调用的工具名称，必须匹配某个 ToolDef 的 name。
    pub name: String,
    /// 工具参数，一个 JSON 对象，由模型根据 ToolDef 的 schema 生成。
    pub arguments: Value,
}

/// 工具执行的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 与 ToolCall 相同的 id，用于关联调用和结果。
    pub id: String,
    /// 工具执行是否成功。
    pub success: bool,
    /// 工具执行的输出，可以是任意 JSON 值。
    /// 如果失败，可以包含错误信息。
    pub output: Value,
}
#[derive(Debug, Error)]
pub enum ToolExecutorError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

// 工具执行器独立
#[async_trait]
pub trait ToolExecutor {
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, ToolExecutorError>;
    fn tools(&self) -> Vec<ToolDef>;
}
