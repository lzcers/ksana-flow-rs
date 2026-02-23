use crate::agent::{ChatCapability, Message, agents::ToolExecutor};

// Agent 结构
pub struct Agent<M: ChatCapability, E: ToolExecutor> {
    model: M,
    tool_executor: E,
    max_iterations: usize,
}
