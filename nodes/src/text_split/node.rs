use async_trait::async_trait;
use flow::{Context, Node, NodeInputs, SendableAny};
use serde_json::Value;

use crate::text_split::{split_text, TextSplitConfig};

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
    async fn run(
        &mut self,
        _ctx: &Context,
        inputs: NodeInputs,
    ) -> Result<Box<dyn SendableAny>, String> {
        let mut config = self.config.clone();
        if let Some(cfg) = inputs.get::<TextSplitConfig>("config") {
            config = cfg.clone();
        } else if let Some(v) = inputs.get::<Value>("config") {
            if let Ok(cfg) = serde_json::from_value::<TextSplitConfig>(v.clone()) {
                config = cfg;
            }
        }

        let input_text = inputs
            .get::<String>("input")
            .or_else(|| inputs.get::<String>("external_start"))
            .or_else(|| inputs.get::<String>("output"))
            .cloned()
            .or_else(|| {
                inputs
                    .iter_any()
                    .find_map(|(_, any)| any.downcast_ref::<String>().cloned())
            })
            .unwrap_or_default();

        let result = split_text(&input_text, &config);
        let out = serde_json::to_value(result).unwrap_or_else(|_| Value::Null);
        Ok(Box::new(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_split::{LineNumberInjectionConfig, TextSplitMode};
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn text_split_node_outputs_segments_json() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let config = TextSplitConfig {
                mode: TextSplitMode::ByLineCount {
                    max_lines_per_part: 2,
                },
                remove_empty_lines: false,
                line_numbers: LineNumberInjectionConfig {
                    enabled: false,
                    template: "{line}: ".to_string(),
                    pad_width: None,
                    pad_char: '0',
                },
                rule_only_keep_matched_ranges: false,
            };
            let mut node = TextSplitNode::new(config);

            let mut map: HashMap<String, Box<dyn flow::SendableAny>> = HashMap::new();
            map.insert("external_start".to_string(), Box::new("a\nb\nc\n".to_string()));
            let payload = node.run(&ctx, NodeInputs::new(map)).await.unwrap();
            let out = {
                fn unwrap_any<'a>(mut any: &'a dyn std::any::Any) -> &'a dyn std::any::Any {
                    loop {
                        let Some(inner) = any.downcast_ref::<Box<dyn flow::SendableAny>>() else {
                            return any;
                        };
                        any = inner.as_ref().as_any();
                    }
                }

                unwrap_any(payload.as_any())
                    .downcast_ref::<Value>()
                    .cloned()
                    .unwrap_or(Value::Null)
            };

            let segments = out
                .get("segments")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].get("text").and_then(|v| v.as_str()), Some("a\nb"));
            assert_eq!(segments[1].get("text").and_then(|v| v.as_str()), Some("c"));
        });
    }
}
