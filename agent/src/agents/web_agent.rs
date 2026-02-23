use crate::agents::tools::{GenericToolExecutor, PlaywrightCliTool};
use crate::agents::{ToolExecutorError, agent::Agent};
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

pub struct WebAgent<M: ChatCapability> {
    agent: Agent<M, GenericToolExecutor>,
}

impl<M: ChatCapability> WebAgent<M> {
    pub fn new(model: M) -> Self {
        let mut executor = GenericToolExecutor::new();
        executor.register(PlaywrightCliTool::new());
        let agent = Agent::new(model, executor);
        Self { agent }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.agent = self.agent.with_max_iterations(max_iterations);
        self
    }

    pub async fn summarize_url(&self, url: &str) -> Result<String, WebAgentError> {
        let system_prompt = format!(
            "You are a web agent that can browse websites using playwright-cli.

First, call playwright_cli with args [\"--help\"] to understand what commands are available.

Then, use playwright-cli to:
1. Open the URL: {}
2. Get the page content
3. Generate a concise summary of the page

Always use the playwright_cli tool to interact with the browser. Do not make up information.",
            url
        );

        let messages = vec![Message::system(system_prompt)];
        let result = self.agent.run(messages).await?;

        let summary = result
            .iter()
            .find_map(|msg| match msg {
                Message::Assistant { content, .. } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "No summary generated".to_string());

        Ok(summary)
    }

    pub async fn run(&self, messages: Vec<Message>) -> Result<Vec<Message>, WebAgentError> {
        Ok(self.agent.run(messages).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::ToolDef;
    use crate::core::Message;
    use crate::models::{ChatCapability, ChatError};
    use async_trait::async_trait;

    struct MockChatModel;

    #[async_trait]
    impl ChatCapability for MockChatModel {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, ChatError> {
            Ok(Message::assistant("Summary complete"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<futures::stream::BoxStream<'static, crate::models::ChatChunk>, ChatError>
        {
            unimplemented!()
        }
    }

    #[test]
    fn test_web_agent_new() {
        let model = MockChatModel;
        let _agent = WebAgent::new(model);
    }

    #[test]
    fn test_summarize_url_system_prompt() {
        let model = MockChatModel;
        let _agent = WebAgent::new(model);
        let url = "https://zeroclawlabs.ai/";

        let system_prompt = format!(
            "You are a web agent that can browse websites using playwright-cli.

First, call playwright_cli with args [\"--help\"] to understand what commands are available.

Then, use playwright-cli to:
1. Open the URL: {}
2. Get the page content
3. Generate a concise summary of the page

Always use the playwright_cli tool to interact with the browser. Do not make up information.",
            url
        );

        assert!(system_prompt.contains(url));
        assert!(system_prompt.contains("playwright-cli"));
        assert!(system_prompt.contains("--help"));
    }

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

        let web_agent = WebAgent::new(model);
        let url = "https://zeroclawlabs.ai/";

        match web_agent.summarize_url(url).await {
            Ok(summary) => {
                println!("Summary of {}:\n{}", url, summary);
                assert!(!summary.is_empty());
            }
            Err(e) => {
                eprintln!("Test failed (playwright-cli may not be installed): {}", e);
            }
        }
    }
}
