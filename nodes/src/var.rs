use async_trait::async_trait;
use flow::{Context, Node, NodeInputs, SendableAny};
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
    T: Clone + Send + Sync + 'static,
    I: Send + Sync,
{
    async fn run(
        &mut self,
        _ctx: &Context,
        _inputs: NodeInputs,
    ) -> Result<Box<dyn SendableAny>, String> {
        Ok(Box::new(self.value.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::Context;
    use flow::NodeInputs;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn test_var_node() {
        fn unwrap_any<'a>(mut any: &'a dyn std::any::Any) -> &'a dyn std::any::Any {
            loop {
                let Some(inner) = any.downcast_ref::<Box<dyn flow::SendableAny>>() else {
                    return any;
                };
                any = inner.as_ref().as_any();
            }
        }

        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();

            // 测试字符串类型
            let mut node: VarNode<String, ()> = VarNode::new("hello".to_string());
            let inputs: HashMap<String, Box<dyn flow::SendableAny>> = HashMap::new();
            let output = node.run(&ctx, NodeInputs::new(inputs)).await.unwrap();
            let s = unwrap_any(output.as_any())
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_default();
            assert_eq!(s, "hello".to_string());

            // 测试整数类型
            let mut int_node: VarNode<i32, ()> = VarNode::new(42i32);
            let inputs: HashMap<String, Box<dyn flow::SendableAny>> = HashMap::new();
            let output = int_node.run(&ctx, NodeInputs::new(inputs)).await.unwrap();
            let n = unwrap_any(output.as_any())
                .downcast_ref::<i32>()
                .copied()
                .unwrap_or_default();
            assert_eq!(n, 42);
        });
    }
}
