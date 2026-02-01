use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;

use super::{splitter::split_text, types::TextSplitConfig};

pub struct TextSplitNode {
    config: TextSplitConfig,
}

impl TextSplitNode {
    pub fn new(config: TextSplitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Node for TextSplitNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let input_text = input
            .get_str_as::<String>("input")
            .or_else(|| input.get_str_as::<String>("external_start"))
            .or_else(|| input.get_str_as::<String>("output"))
            .or_else(|| input.get_any_as::<String>())
            .unwrap_or_default();

        let result = split_text(&input_text, &self.config)
            .segments
            .into_iter()
            .map(|s| s.text)
            .collect::<Vec<String>>();
        let out = serde_json::to_value(result).unwrap_or_else(|_| Value::Null);
        Ok(out.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        TextSplitRule,
        text::types::{LineNumberInjectionConfig, TextSplitMode},
    };

    use super::super::{text_split_node::*, types::TextSplitConfig};
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn text_split_node_outputs_segments_json() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let config = TextSplitConfig {
                mode: TextSplitMode::ByRule {
                    rule: TextSplitRule::HeadingKeywords {
                        keywords: vec!["集".to_string()],
                        require_prefix: Some("第".to_string()),
                    },
                },
                remove_empty_lines: false,
                line_numbers: LineNumberInjectionConfig {
                    enabled: true,
                    template: "{line}: ".to_string(),
                    pad_width: None,
                    pad_char: '0',
                },
                rule_only_keep_matched_ranges: false,
            };
            let mut node = TextSplitNode::new(config);
            let text = r#""#;
            let mut map: HashMap<String, Value> = HashMap::new();
            map.insert(
                "external_start".to_string(),
                Value::String(text.to_string()),
            );
            let out = node
                .run(&ctx, &Input::new(map))
                .await
                .unwrap()
                .get()
                .cloned()
                .unwrap_or(Value::Null);

            let segments = out
                .get("segments")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            segments.iter().for_each(|s| {
                println!("{}", s["text"]);
                println!("------------------------------------")
            });
        });
    }
}
