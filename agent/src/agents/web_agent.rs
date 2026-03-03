use crate::agents::tools::{GenericToolExecutor, PlaywrightCliTool, ToolRegistry};
use crate::agents::{ToolExecutor, ToolExecutorError, agent::Agent};
use crate::core::Message;
use crate::models::ChatCapability;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebAgentError {
    #[error("Agent error: {0}")]
    Agent(#[from] crate::agents::agent::AgentError),
    #[error("Tool executor error: {0}")]
    Tool(#[from] ToolExecutorError),
}

pub struct WebAgent<M: ChatCapability, E: ToolExecutor> {
    agent: Agent<M, E>,
}

impl<M: ChatCapability + Send + 'static> WebAgent<M, GenericToolExecutor> {
    /// 创建一个带有默认 PlaywrightCliTool 的 WebAgent
    pub fn new(model: M) -> Self {
        let mut executor = GenericToolExecutor::new();
        executor.register(PlaywrightCliTool::new());
        let agent = Agent::new(model, executor);
        Self { agent }
    }

    /// 从 ToolRegistry 创建 WebAgent
    pub fn with_registry(model: M, registry: ToolRegistry) -> Self {
        let executor = GenericToolExecutor::with_registry(registry);
        let agent = Agent::new(model, executor);
        Self { agent }
    }
}

impl<M: ChatCapability + Send + 'static, E: ToolExecutor + Send + 'static> WebAgent<M, E> {
    /// 使用任意 ToolExecutor 创建 WebAgent
    pub fn with_executor(model: M, executor: E) -> Self {
        let agent = Agent::new(model, executor);
        Self { agent }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.agent = self.agent.with_max_iterations(max_iterations);
        self
    }

    /// 执行一个自然语言描述的网页浏览任务
    pub async fn execute_task(&self, task_description: &str) -> Result<String, WebAgentError> {
        let system_prompt = r#"You are a capable web browsing agent that can interact with websites using the playwright_cli tool.

Your workflow:
1. Understand the user's task
2. Use playwright_cli commands to navigate and interact with web pages
3. Extract relevant information
4. Provide a helpful response to the user

Guidelines:
- ALWAYS use playwright_cli to interact with browsers - do not make up information
- Break complex tasks into multiple steps
- After gathering information, provide a comprehensive summary to the user"#.to_string();

        let messages = vec![
            Message::system(system_prompt),
            Message::user(task_description.to_string()),
        ];
        let result = self.agent.run(messages).await?;

        let response = result
            .iter()
            .rev()
            .find_map(|msg| match msg {
                Message::Assistant { content, .. } if !content.is_empty() => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "No response generated".to_string());

        Ok(response)
    }

    /// 运行自定义消息序列
    pub async fn run(&self, messages: Vec<Message>) -> Result<Vec<Message>, WebAgentError> {
        Ok(self.agent.run(messages).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires playwright-cli installed and a real LLM provider configured"]
    async fn test_summarize_zeroclaw_readme() {
        dotenv::dotenv().ok();

        let provider = match crate::providers::DeepSeekProvider::from_env() {
            Ok(p) => std::sync::Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping integration test");
                return;
            }
        };

        let mut model = crate::models::ChatModel::new();
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], provider);

        if let Err(e) = model.set_active_model("deepseek-chat") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let web_agent: WebAgent<crate::models::ChatModel, GenericToolExecutor> =
            WebAgent::new(model);
        let url = "https://zeroclawlabs.ai/";

        let result = web_agent
            .execute_task(&format!(
                "Please browse to {} and provide a concise summary of the page content.",
                url
            ))
            .await;
        assert!(result.is_ok(), "Failed to execute task: {:?}", result);
    }
}
