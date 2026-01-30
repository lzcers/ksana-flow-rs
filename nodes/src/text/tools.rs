use super::{splitter::split_text, types::*};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

impl Tool for TextSplitTool {
    const NAME: &'static str = "text_split";
    type Error = TextSplitToolError;
    type Args = TextSplitToolArgs;
    type Output = TextSplitResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "将文本按行数或规则分割成多个片段，支持行号注入".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "要分割的文本内容"
                    },
                    "config": {
                        "type": "object",
                        "description": "分割配置",
                        "properties": {
                            "mode": {
                                "type": "object",
                                "description": "分割模式",
                                "properties": {
                                    "by_line_count": {
                                        "type": "object",
                                        "description": "按行数分割",
                                        "properties": {
                                            "max_lines_per_part": {
                                                "type": "integer",
                                                "description": "每个片段的最大行数"
                                            }
                                        }
                                    },
                                    "by_rule": {
                                        "type": "object",
                                        "description": "按规则分割",
                                        "properties": {
                                            "rule": {
                                                "type": "object",
                                                "properties": {
                                                    "heading_keywords": {
                                                        "type": "object",
                                                        "properties": {
                                                            "keywords": {
                                                                "type": "array",
                                                                "items": { "type": "string" },
                                                                "description": "标题关键词列表"
                                                            },
                                                            "require_prefix": {
                                                                "type": ["string", "null"],
                                                                "description": "必需的前缀"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "remove_empty_lines": {
                                "type": "boolean",
                                "description": "是否移除空行"
                            },
                            "line_numbers": {
                                "type": "object",
                                "description": "行号注入配置",
                                "properties": {
                                    "enabled": {
                                        "type": "boolean",
                                        "description": "是否启用行号注入"
                                    },
                                    "template": {
                                        "type": "string",
                                        "description": "行号模板，使用{line}占位符"
                                    },
                                    "pad_width": {
                                        "type": ["integer", "null"],
                                        "description": "行号填充宽度"
                                    },
                                    "pad_char": {
                                        "type": "string",
                                        "description": "行号填充字符"
                                    }
                                }
                            },
                            "rule_only_keep_matched_ranges": {
                                "type": "boolean",
                                "description": "仅保留匹配的范围（按规则分割时）"
                            }
                        }
                    }
                },
                "required": ["text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = split_text(&args.text, &args.config);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::types::{LineNumberInjectionConfig, TextSplitMode};

    #[tokio::test]
    async fn test_text_split_tool() {
        let tool = TextSplitTool::new();
        let args = TextSplitToolArgs {
            text: "a\nb\nc\n".to_string(),
            config: TextSplitConfig {
                mode: TextSplitMode::ByLineCount {
                    max_lines_per_part: 2,
                },
                remove_empty_lines: false,
                line_numbers: LineNumberInjectionConfig::default(),
                rule_only_keep_matched_ranges: false,
            },
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.total_lines, 3);
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].text, "a\nb");
        assert_eq!(result.segments[1].text, "c");
    }
}
