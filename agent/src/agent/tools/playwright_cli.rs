use crate::{
    agent::tools::Tool,
    agent::{ToolDef, ToolExecutorError},
};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub struct PlaywrightCliTool {
    definition: ToolDef,
}

impl PlaywrightCliTool {
    pub fn new() -> Self {
        let definition = ToolDef {
            name: "playwright-cli".to_string(),
            description: r#"
            A browser automation tool for web navigation, content extraction, and interaction.
            Common commands:
            - `open <url>` - Navigate to a URL and wait for page load
            - `--help` - Show all available commands
            Strict execution rules (must follow in order):
            First run: Always start with open <url> to navigate to the target URL and wait for page load.
            Second run: Immediately execute --help to list all valid commands, parameters, and usage.
            Subsequent operations:
            - Only use commands explicitly shown in the --help output.
            - Never use any command not present in --help.
            - Do not guess, invent, or run any command blindly.
            "#
            .trim()
            .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command and arguments, e.g.: [\"open\", \"https://example.com\"], [\"content\"], [\"text\"], [\"click\", \".button\"]"
                    }
                },
                "required": ["args"]
            }),
        };

        Self { definition }
    }

    async fn execute_command(&self, args: &[String]) -> Result<String, ToolExecutorError> {
        let (cmd, cmd_args) = if cfg!(windows) {
            ("cmd.exe", {
                let mut full_args = vec!["/c".to_string(), "playwright-cli".to_string()];
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
                println!("playwright_cli error: {:?}", e);
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

impl Default for PlaywrightCliTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PlaywrightCliTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
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

        Ok(serde_json::json!({ "stdout": output }))
    }
}
