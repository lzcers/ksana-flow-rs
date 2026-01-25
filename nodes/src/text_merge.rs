use async_trait::async_trait;
use flow::{Context, Node, NodeInputs};
use std::collections::BTreeMap;
use tracing::info;

/// 将多个文本输入合并为一个字符串输出。
///
/// 默认采用追加模式（空分隔符），也可以指定分隔符。
/// 合并顺序按照输入节点的 ID 字母顺序排列。
pub struct TextMergeNode {
    separator: String,
}

impl TextMergeNode {
    /// 创建一个新的 TextMergeNode。
    ///
    /// # Arguments
    ///
    /// * `separator` - 可选的分隔符。如果为 None，则默认为空字符串。
    pub fn new(separator: Option<String>) -> Self {
        Self {
            separator: separator.unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Node for TextMergeNode {
    type Out = String;

    async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> Self::Out {
        // 使用 BTreeMap 来按 NodeId 排序，确保合并顺序确定
        let sorted_inputs: BTreeMap<_, _> = inputs.inputs.iter().collect();
        let parts: Vec<&str> = sorted_inputs
            .values()
            .filter_map(|any| any.as_any().downcast_ref::<String>().map(|s| s.as_str()))
            .collect();
        info!(
            "sorted_inputs: {:?} input keys {:?}",
            parts,
            sorted_inputs.keys()
        );

        parts.join(&self.separator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::SendableAny;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    // Helper to create inputs
    fn create_inputs(data: Vec<(&str, &str)>) -> NodeInputs {
        let mut inputs = HashMap::new();
        for (id, val) in data {
            let boxed_val: Box<dyn SendableAny> = Box::new(val.to_string());
            inputs.insert(id.to_string(), boxed_val);
        }
        NodeInputs::new(inputs)
    }

    #[test]
    fn test_text_merge_node_default() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(None);

            let inputs = create_inputs(vec![("a", "Hello"), ("b", "World")]);
            let output = node.run(&ctx, inputs).await;

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
            let output = node.run(&ctx, inputs).await;

            assert_eq!(output, "This, is, test");
        });
    }

    #[test]
    fn test_text_merge_node_mixed_types() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextMergeNode::new(Some(" ".to_string()));

            let mut inputs_map = HashMap::new();
            inputs_map.insert(
                "a".to_string(),
                Box::new("Hello".to_string()) as Box<dyn SendableAny>,
            );
            inputs_map.insert(
                "b".to_string(),
                Box::new(42) as Box<dyn SendableAny>, // Should be ignored
            );
            inputs_map.insert(
                "c".to_string(),
                Box::new("World".to_string()) as Box<dyn SendableAny>,
            );
            let inputs = NodeInputs::new(inputs_map);

            let output = node.run(&ctx, inputs).await;

            assert_eq!(output, "Hello World");
        });
    }
}
