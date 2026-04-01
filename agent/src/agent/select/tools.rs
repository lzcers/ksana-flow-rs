use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::filesystem::FsSelector;
use crate::agent::select::{
    FileSelector, GrepRequest, ListDirRequest, ReadFileRequest, SelectConfig,
};
use crate::agent::{GenericToolExecutor, Tool, ToolDef, ToolExecutorError};

#[derive(Debug, Clone)]
pub struct SelectToolConfig {
    pub select: SelectConfig,
}

impl Default for SelectToolConfig {
    fn default() -> Self {
        Self {
            select: SelectConfig::default(),
        }
    }
}

impl SelectToolConfig {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            select: SelectConfig::new(workspace_root),
        }
    }

    pub fn from_config(select: SelectConfig) -> Self {
        Self { select }
    }
}

#[derive(Debug, Clone)]
struct ToolState {
    selector: Arc<FsSelector>,
}

impl ToolState {
    fn new(config: SelectToolConfig) -> Self {
        Self {
            selector: Arc::new(FsSelector::new(config.select)),
        }
    }
}

pub fn register_select_tools(executor: &mut GenericToolExecutor, config: SelectToolConfig) {
    let state = ToolState::new(config);
    executor.register(FileListTool::new(state.clone()));
    executor.register(FileSearchTool::new(state.clone()));
    executor.register(FileReadTool::new(state));
}

pub struct FileListTool {
    definition: ToolDef,
    state: ToolState,
}

impl FileListTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "file_list".to_string(),
                description: "List files and directories within the configured workspace."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Workspace-relative directory path." },
                        "max_depth": { "type": "integer", "minimum": 1 },
                        "pattern": { "type": "string", "description": "Optional glob for file names." }
                    },
                    "required": ["path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for FileListTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let mut request = ListDirRequest::new(path);
        request.max_depth = optional_usize(&arguments, "max_depth")?;
        request.pattern = optional_string(&arguments, "pattern")?;
        let entries = self.state.selector.list_dir(&request).map_err(exec_err)?;
        Ok(json!({ "entries": entries }))
    }
}

pub struct FileSearchTool {
    definition: ToolDef,
    state: ToolState,
}

impl FileSearchTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "file_search".to_string(),
                description: "Search file contents in the workspace using a regex pattern."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "Regex pattern to search for." },
                        "path": { "type": "string", "description": "Workspace-relative file or directory path." },
                        "file_pattern": { "type": "string", "description": "Optional glob filter for file names." },
                        "ignore_case": { "type": "boolean" },
                        "max_results": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["pattern", "path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for FileSearchTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let pattern = required_string(&arguments, "pattern")?;
        let path = required_string(&arguments, "path")?;
        let mut request = GrepRequest::new(pattern, path);
        request.file_pattern = optional_string(&arguments, "file_pattern")?;
        request.ignore_case = optional_bool(&arguments, "ignore_case")?.unwrap_or(false);
        request.max_results = optional_usize(&arguments, "max_results")?;
        let matches = self.state.selector.grep(&request).map_err(exec_err)?;
        Ok(json!({ "matches": matches }))
    }
}

pub struct FileReadTool {
    definition: ToolDef,
    state: ToolState,
}

impl FileReadTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "file_read".to_string(),
                description:
                    "Read a file fragment from the workspace using optional 1-based line bounds."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Workspace-relative file path." },
                        "start_line": { "type": "integer", "minimum": 1 },
                        "end_line": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let mut request = ReadFileRequest::new(path);
        request.start_line = optional_usize(&arguments, "start_line")?;
        request.end_line = optional_usize(&arguments, "end_line")?;
        let content = self.state.selector.read_file(&request).map_err(exec_err)?;
        serde_json::to_value(content).map_err(exec_err)
    }
}

fn exec_err(error: impl std::fmt::Display) -> ToolExecutorError {
    ToolExecutorError::ExecutionError(error.to_string())
}

fn required_string(arguments: &Value, key: &str) -> Result<String, ToolExecutorError> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            ToolExecutorError::ExecutionError(format!("Missing or invalid '{key}' parameter"))
        })
}

fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>, ToolExecutorError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|v| Some(v.to_string()))
            .ok_or_else(|| ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))),
    }
}

fn optional_usize(arguments: &Value, key: &str) -> Result<Option<usize>, ToolExecutorError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let number = value.as_u64().ok_or_else(|| {
                ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))
            })?;
            usize::try_from(number).map(Some).map_err(|_| {
                ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))
            })
        }
    }
}

fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>, ToolExecutorError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))),
    }
}
