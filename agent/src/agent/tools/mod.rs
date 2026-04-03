use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod ask_user;
pub mod playwright_cli;
pub mod registry;

pub use playwright_cli::PlaywrightCliTool;
pub use registry::{GenericToolExecutor, Tool, ToolRegistry};
use thiserror::Error;

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

/// OpenAI 兼容的工具调用 function 字段
///
/// 注意：流式响应中，增量 chunks 可能只包含部分字段，
/// 使用 `#[serde(default)]` 允许缺失时使用默认值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

/// 模型发起的工具调用请求（OpenAI 兼容格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用的唯一标识符，用于将结果与调用关联。
    /// 通常由模型生成，执行结果需原样返回。
    ///
    /// 注意：流式响应中，后续增量 chunks 不包含 id 字段，
    /// 使用 `#[serde(default)]` 允许缺失时使用空字符串作为默认值。
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    /// OpenAI 嵌套格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolCallFunction>,
    /// 简化格式（用于内部使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl ToolCall {
    pub fn get_name(&self) -> String {
        if let Some(function) = &self.function {
            function.name.clone()
        } else if let Some(name) = &self.name {
            name.clone()
        } else {
            String::new()
        }
    }

    pub fn get_arguments(&self) -> Value {
        if let Some(function) = &self.function {
            serde_json::from_str(&function.arguments).unwrap_or(Value::Null)
        } else if let Some(args) = &self.arguments {
            args.clone()
        } else {
            Value::Null
        }
    }
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

#[derive(Debug, Error, Clone)]
pub enum ToolExecutorError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

// 工具执行器独立
#[async_trait]
pub trait ToolExecutor: Sync {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolExecutorError>;
    fn tools(&self) -> &Vec<ToolDef>;
}
