use async_trait::async_trait;
use flow::{Context, Node};
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
    T: Clone + Send + Sync,
    I: Send + Sync,
{
    type In = I;
    type Out = T;

    async fn run(&mut self, _ctx: &Context, _input: Self::In) -> Self::Out {
        self.value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::Context;
    use tokio::runtime::Runtime;

    #[test]
    fn test_var_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();

            // 测试字符串类型
            let mut node = VarNode::new("hello".to_string());
            let output = node.run(&ctx, ()).await;
            assert_eq!(output, "hello".to_string());

            // 测试整数类型
            let mut int_node = VarNode::new(42i32);
            let output = int_node.run(&ctx, ()).await;
            assert_eq!(output, 42);
        });
    }
}
