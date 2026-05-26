use super::types::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 文本分割工具的输入参数
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextSplitToolArgs {
    /// 要分割的文本内容
    pub text: String,
    /// 分割模式配置
    #[serde(default)]
    pub config: TextSplitConfig,
}

/// 文本分割工具的错误类型
#[derive(Debug, Error)]
pub enum TextSplitToolError {
    #[error("Text split error: {0}")]
    SplitError(String),
}

/// 文本分割工具结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextSplitTool;

impl TextSplitTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextSplitTool {
    fn default() -> Self {
        Self::new()
    }
}
