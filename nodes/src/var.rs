use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde::Serialize;
use serde_json::Value;
use std::marker::PhantomData;

/// 一个泛型的变量节点，返回预设的值。
/// 它会忽略输入，并返回在创建时提供的数值。
pub struct VarNode<T, I = ()> {
    value: T,
    _phantom: PhantomData<I>,
}

impl<T, I> VarNode<T, I> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<T, I> Node for VarNode<T, I>
where
    T: Serialize + Clone + Send + Sync + 'static,
    I: Send + Sync,
{
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let v = serde_json::to_value(self.value.clone()).map_err(|e| format!("VarNode: {}", e))?;
        Ok(v.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn test_var_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();

            // 测试字符串类型
            let mut node: VarNode<String, ()> = VarNode::new("hello".to_string());
            let inputs: HashMap<String, Value> = HashMap::new();
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get(), Some(&Value::String("hello".to_string())));

            // 测试整数类型
            let mut int_node: VarNode<i32, ()> = VarNode::new(42i32);
            let inputs: HashMap<String, Value> = HashMap::new();
            let output = int_node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get(), Some(&serde_json::json!(42)));
        });
    }
}
