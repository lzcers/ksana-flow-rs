use crate::agent::{ChatCapability, agents::ToolExecutor};

struct Agent<M: ChatCapability, E: ToolExecutor> {
    model: M,
    tool_executor: E,
}
