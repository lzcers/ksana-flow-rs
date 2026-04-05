use crate::agent::{Tool, ToolDef, ToolExecutorError};
use async_trait::async_trait;
use serde_json::Value;

pub struct AskUserTool {
    def: ToolDef,
}

impl AskUserTool {
    pub fn new() -> Self {
        todo!()
    }
}

#[async_trait]
impl Tool for AskUserTool {
    fn definition(&self) -> &ToolDef {
        &self.def
    }
    async fn execute(&self, args: Value) -> Result<Value, ToolExecutorError> {
        todo!()
    }
}
