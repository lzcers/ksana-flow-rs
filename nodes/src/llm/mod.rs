use flow::AnyNode;
use std::sync::Arc;
use tokio::sync::RwLock;

mod input;
mod llm;
mod llm_stream;

pub use llm::LLMNode;
pub(crate) use llm_stream::LLMStreamObservable;

pub fn create_llm_any_node(
    system_prompt: &str,
    user_prompt_template: &str,
    model: &str,
    stream: bool,
) -> Arc<RwLock<dyn AnyNode>> {
    if stream {
        let node = llm_stream::LLMStreamNode::new(system_prompt, user_prompt_template, model);
        Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>
    } else {
        let node = llm::LLMNode::new(system_prompt, user_prompt_template, model);
        Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>
    }
}
