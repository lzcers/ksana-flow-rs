use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;

/// 将多个文本输入合并为一个字符串输出。
///
/// 默认采用追加模式（空分隔符），也可以指定分隔符。
/// 合并顺序按照输入节点的 ID 字母顺序排列。
pub struct TextMergeNode {
    separator: String,
}

fn unescape_separator(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            out.push('\\');
            break;
        };

        match escaped {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            'u' => {
                let mut hex = String::with_capacity(4);
                for _ in 0..4 {
                    if let Some(h) = chars.next() {
                        hex.push(h);
                    } else {
                        break;
                    }
                }

                if hex.len() == 4 {
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code) {
                            out.push(c);
                            continue;
                        }
                    }
                }

                out.push('\\');
                out.push('u');
                out.push_str(&hex);
            }
            other => {
                out.push('\\');
                out.push(other);
            }
        }
    }

    out
}

impl TextMergeNode {
    /// 创建一个新的 TextMergeNode。
    ///
    /// # Arguments
    ///
    /// * `separator` - 可选的分隔符。如果为 None，则默认为空字符串。
    pub fn new(separator: Option<String>) -> Self {
        Self {
            separator: separator
                .as_deref()
                .map(unescape_separator)
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Node for TextMergeNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        fn extract_text<'a>(v: &'a Value) -> Option<&'a str> {
            match v {
                Value::String(s) => Some(s.as_str()),
                Value::Object(map) => map.get("output").and_then(|v| v.as_str()),
                _ => None,
            }
        }

        let mut entries: Vec<_> = input.get_values().iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        let parts: Vec<&str> = entries
            .iter()
            .filter_map(|(_, v)| extract_text(v))
            .collect();
        Ok(Value::String(parts.join(&self.separator)).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    // Helper to create inputs
    fn create_inputs(data: Vec<(&str, &str)>) -> Input {
        let mut inputs: HashMap<String, Value> = HashMap::new();
        for (id, val) in data {
            inputs.insert(id.to_string(), Value::String(val.to_string()));
        }
        Input::new(inputs)
    }

    fn extract_string(out: Output) -> String {
        out.get_as::<String>().unwrap_or_default()
    }

    #[test]
    fn test_text_merge_node_default() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(None);

            let inputs = create_inputs(vec![("a", "Hello"), ("b", "World")]);
            let output = extract_string(node.run(&ctx, &inputs).await.unwrap());

            assert_eq!(output, "HelloWorld");
        });
    }

    #[test]
    fn test_text_merge_node_with_separator() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(Some(", ".to_string()));

            // Intentionally unordered in vector, but NodeInputs is HashMap
            // The node should sort by key "a", "b", "c"
            let inputs = create_inputs(vec![("b", "is"), ("a", "This"), ("c", "test")]);
            let output = extract_string(node.run(&ctx, &inputs).await.unwrap());

            assert_eq!(output, "This, is, test");
        });
    }

    #[test]
    fn test_text_merge_node_mixed_types() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(Some(" ".to_string()));

            let mut inputs_map: HashMap<String, Value> = HashMap::new();
            inputs_map.insert("a".to_string(), Value::String("Hello".to_string()));
            inputs_map.insert("b".to_string(), json!(42));
            inputs_map.insert("c".to_string(), Value::String("World".to_string()));
            let inputs = Input::new(inputs_map);

            let output = extract_string(node.run(&ctx, &inputs).await.unwrap());

            assert_eq!(output, "Hello World");
        });
    }

    #[test]
    fn test_text_merge_node_json_string_variants() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(Some(" ".to_string()));

            let mut inputs_map: HashMap<String, Value> = HashMap::new();
            inputs_map.insert("a".to_string(), json!("Hello"));
            inputs_map.insert("b".to_string(), json!({ "output": "World" }));
            let inputs = Input::new(inputs_map);

            let output = extract_string(node.run(&ctx, &inputs).await.unwrap());
            assert_eq!(output, "Hello World");
        });
    }

    #[test]
    fn test_text_merge_node_separator_escape_newline() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(Some("\\n".to_string()));

            let inputs = create_inputs(vec![("a", "Hello"), ("b", "World")]);
            let output = extract_string(node.run(&ctx, &inputs).await.unwrap());

            assert_eq!(output, "Hello\nWorld");
        });
    }
}
