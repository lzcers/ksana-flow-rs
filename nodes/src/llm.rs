use flow::Node;
use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::deepseek::{self, CompletionModel},
};
use tokio::{runtime::Handle, task};
struct LLMNode {
    llm: Agent<CompletionModel>,
}

impl LLMNode {
    fn new() -> Self {
        dotenv::dotenv().ok();
        let client = deepseek::Client::from_env();
        let llm = client.agent("deepseek-chat").build();
        Self { llm }
    }
}
impl Node for LLMNode {
    type In = String;

    type Out = String;

    fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        let handle = Handle::current();
        task::block_in_place(|| {
            handle.block_on(async { self.llm.prompt(&input).await.expect("LLM prompt failed") })
        })
    }
}

#[cfg(test)]
mod tests {
    use flow::Context;
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_llm_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new();
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);
            let output = node.run(&ctx, input);
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }
}
