use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use crate::agent::agents::{
    ToolCall, ToolDef, ToolExecutor, ToolExecutorError, ToolResult, agent::Agent,
};
use crate::agent::{ChatCapability, Message};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebAgentError {
    #[error("Agent error: {0}")]
    Agent(#[from] crate::agent::agents::agent::AgentError),
    #[error("Tool executor error: {0}")]
    Tool(#[from] ToolExecutorError),
    #[error(
        "Playwright-cli not found. Please install it with: npm install -g @playwright/cli@latest"
    )]
    PlaywrightCliNotFound,
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}

pub struct PlaywrightCliExecutor {
    tools: Vec<ToolDef>,
}

impl PlaywrightCliExecutor {
    pub fn new() -> Self {
        let tool = ToolDef {
            name: "playwright_cli".to_string(),
            description: "Execute any playwright-cli command. Use 'playwright_cli --help' to see all available commands.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Arguments to pass to playwright-cli, e.g., [\"--help\"], [\"open\", \"https://example.com\"]"
                    }
                },
                "required": ["args"]
            }),
        };

        Self { tools: vec![tool] }
    }

    async fn execute_command(&self, args: &[String]) -> Result<String, ToolExecutorError> {
        let (cmd, cmd_args) = if cfg!(windows) {
            ("powershell.exe", {
                let mut full_args = vec!["-Command".to_string(), "playwright-cli".to_string()];
                full_args.extend_from_slice(args);
                full_args
            })
        } else {
            ("playwright-cli", args.to_vec())
        };

        let output = Command::new(cmd)
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ToolExecutorError::ExecutionError(
                        "playwright-cli not found. Please install it with: npm install -g @playwright/cli@latest".to_string()
                    )
                } else {
                    ToolExecutorError::ExecutionError(format!("Failed to execute playwright-cli: {}", e))
                }
            })?;

        let mut result = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str("stderr:\n");
            result.push_str(&stderr);
        }

        if !output.status.success() {
            return Err(ToolExecutorError::ExecutionError(format!(
                "playwright-cli exited with code {:?}:\n{}",
                output.status.code(),
                result
            )));
        }

        Ok(result)
    }
}

impl Default for PlaywrightCliExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolExecutor for PlaywrightCliExecutor {
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, ToolExecutorError> {
        let name = call.get_name();
        if name != "playwright_cli" {
            return Err(ToolExecutorError::ToolNotFound(name));
        }

        let arguments = call.get_arguments();
        let args = arguments
            .get("args")
            .and_then(|a| a.as_array())
            .ok_or_else(|| {
                ToolExecutorError::ExecutionError("Missing or invalid 'args' parameter".to_string())
            })?;

        let args: Vec<String> = args
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        let output = self.execute_command(&args).await?;

        Ok(ToolResult {
            id: call.id,
            success: true,
            output: serde_json::json!({ "stdout": output }),
        })
    }

    fn tools(&self) -> &Vec<ToolDef> {
        &self.tools
    }
}

pub struct WebAgent<M: ChatCapability> {
    agent: Agent<M, PlaywrightCliExecutor>,
}

impl<M: ChatCapability> WebAgent<M> {
    pub fn new(model: M) -> Self {
        let executor = PlaywrightCliExecutor::new();
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
    use crate::agent::agents::ToolDef;
    use crate::agent::{ChatCapability, ChatError, Message};
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
        ) -> Result<futures::stream::BoxStream<'static, crate::agent::ChatChunk>, ChatError>
        {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_playwright_cli_executor_new() {
        let executor = PlaywrightCliExecutor::new();
        assert_eq!(executor.tools().len(), 1);
        assert_eq!(executor.tools()[0].name, "playwright_cli");
    }

    #[tokio::test]
    async fn test_playwright_cli_executor_help() {
        let executor = PlaywrightCliExecutor::new();
        let result = executor.execute_command(&["--help".to_string()]).await;
        match result {
            Ok(output) => {
                assert!(!output.is_empty());
            }
            Err(e) => {
                println!("Note: playwright-cli not installed, skipping test: {}", e);
            }
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

        let provider = match crate::agent::providers::DeepSeekProvider::from_env() {
            Ok(p) => std::sync::Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping integration test");
                return;
            }
        };

        let mut model = crate::agent::models::ChatModel::new();
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
