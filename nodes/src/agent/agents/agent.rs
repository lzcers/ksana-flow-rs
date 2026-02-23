use futures::future::join_all;
use thiserror::Error;

use crate::agent::{
    ChatCapability, ChatError, Message,
    agents::{ToolCall, ToolExecutor, ToolExecutorError, ToolResult},
};

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Chat error: {0}")]
    Chat(#[from] ChatError),
    #[error("Tool execution error: {0}")]
    Tool(#[from] ToolExecutorError),
    #[error("Max iterations reached: {0}")]
    MaxIterationsReached(usize),
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

// Agent 结构
pub struct Agent<M: ChatCapability, E: ToolExecutor> {
    model: M,
    tool_executor: E,
    max_iterations: usize,
}

impl<M: ChatCapability, E: ToolExecutor> Agent<M, E> {
    pub fn new(model: M, tool_executor: E) -> Self {
        Self {
            model,
            tool_executor,
            max_iterations: 10,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub async fn run(&self, mut messages: Vec<Message>) -> Result<Vec<Message>, AgentError> {
        let tools = self.tool_executor.tools();

        for iteration in 0..self.max_iterations {
            let response = self
                .model
                .chat(messages.clone(), Some(tools.clone()))
                .await?;

            messages.push(response.clone());

            let tool_calls = match &response {
                Message::Assistant { tool_calls, .. } => tool_calls,
                _ => {
                    return Err(AgentError::InvalidMessage(
                        "Expected assistant message".to_string(),
                    ));
                }
            };

            let Some(tool_calls) = tool_calls else {
                break;
            };

            if tool_calls.is_empty() {
                break;
            }

            let tool_results = self.execute_tools(tool_calls).await?;

            for tool_result in tool_results {
                messages.push(Message::Tool {
                    tool_call_id: tool_result.id.clone(),
                    content: serde_json::to_string(&tool_result.output).unwrap_or_default(),
                });
            }

            if iteration + 1 >= self.max_iterations {
                return Err(AgentError::MaxIterationsReached(self.max_iterations));
            }
        }

        Ok(messages)
    }

    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Result<Vec<ToolResult>, AgentError> {
        let futures = tool_calls
            .iter()
            .map(|call| self.tool_executor.execute(call.clone()));

        let results = join_all(futures).await;

        let mut tool_results = Vec::with_capacity(results.len());
        for result in results {
            tool_results.push(result?);
        }

        Ok(tool_results)
    }
}
