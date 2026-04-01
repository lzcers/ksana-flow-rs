use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::{ToolCall, ToolDef, ToolExecutor, ToolExecutorError, ToolResult};

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDef;
    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    tool_defs: Vec<ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            tool_defs: Vec::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let def = tool.definition().clone();
        let name = def.name.clone();
        self.tools.insert(name, Arc::new(tool));
        self.tool_defs.push(def);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn tool_defs(&self) -> &Vec<ToolDef> {
        &self.tool_defs
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GenericToolExecutor {
    registry: ToolRegistry,
}

impl GenericToolExecutor {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.registry.register(tool);
    }
}

impl Default for GenericToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolExecutor for GenericToolExecutor {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolExecutorError> {
        let name = call.get_name();
        let tool = self
            .registry
            .get(&name)
            .ok_or_else(|| ToolExecutorError::ToolNotFound(name.clone()))?;

        let arguments = call.get_arguments();
        let output = tool.execute(arguments).await?;

        Ok(ToolResult {
            id: call.id.clone(),
            success: true,
            output,
        })
    }

    fn tools(&self) -> &Vec<ToolDef> {
        self.registry.tool_defs()
    }
}
