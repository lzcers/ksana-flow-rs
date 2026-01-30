use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSplitMode {
    ByLineCount { max_lines_per_part: usize },
    ByRule { rule: TextSplitRule },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSplitRule {
    HeadingKeywords {
        keywords: Vec<String>,
        require_prefix: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineNumberInjectionConfig {
    pub enabled: bool,
    pub template: String,
    pub pad_width: Option<usize>,
    pub pad_char: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSplitConfig {
    pub mode: TextSplitMode,
    pub remove_empty_lines: bool,
    pub line_numbers: LineNumberInjectionConfig,
    pub rule_only_keep_matched_ranges: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSegment {
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSplitResult {
    pub total_lines: usize,
    pub segments: Vec<TextSegment>,
}

impl Default for LineNumberInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            template: "{line}: ".to_string(),
            pad_width: None,
            pad_char: '0',
        }
    }
}

impl Default for TextSplitConfig {
    fn default() -> Self {
        Self {
            mode: TextSplitMode::ByLineCount {
                max_lines_per_part: 200,
            },
            remove_empty_lines: false,
            line_numbers: LineNumberInjectionConfig::default(),
            rule_only_keep_matched_ranges: false,
        }
    }
}
