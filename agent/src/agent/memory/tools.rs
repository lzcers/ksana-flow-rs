use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::filesystem::FsMemoryStore;
use crate::agent::memory::{LineRange, MemoryConfig, MemoryError, MemoryStore, MemoryView};
use crate::agent::{GenericToolExecutor, Tool, ToolDef, ToolExecutorError};

#[derive(Debug, Clone)]
pub struct MemoryToolConfig {
    pub memory: MemoryConfig,
}

impl Default for MemoryToolConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
        }
    }
}

impl MemoryToolConfig {
    pub fn new(memory_base_path: impl Into<PathBuf>) -> Self {
        Self {
            memory: MemoryConfig::new(memory_base_path),
        }
    }

    pub fn from_config(memory: MemoryConfig) -> Self {
        Self { memory }
    }
}

#[derive(Debug, Clone)]
struct ToolState {
    memory: Arc<Mutex<FsMemoryStore>>,
}

impl ToolState {
    fn new(config: MemoryToolConfig) -> Result<Self, MemoryError> {
        Ok(Self {
            memory: Arc::new(Mutex::new(FsMemoryStore::new(config.memory)?)),
        })
    }
}

pub fn register_memory_tools(
    executor: &mut GenericToolExecutor,
    config: MemoryToolConfig,
) -> Result<(), MemoryError> {
    let state = ToolState::new(config)?;
    executor.register(MemoryReadTool::new(state.clone()));
    executor.register(MemoryWriteTool::new(state.clone()));
    executor.register(MemoryUpdateTool::new(state.clone()));
    executor.register(MemoryDeleteTool::new(state.clone()));
    executor.register(MemoryRenameTool::new(state.clone()));
    executor.register(MemoryInsertTool::new(state));
    Ok(())
}

pub struct MemoryReadTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryReadTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_read".to_string(),
                description: "Read a file or list a directory under /memories. Supports optional 1-based start_line and end_line for file views.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path under /memories." },
                        "start_line": { "type": "integer", "minimum": 1, "description": "Optional 1-based start line for file reads." },
                        "end_line": { "type": "integer", "minimum": 1, "description": "Optional 1-based end line for file reads." }
                    },
                    "required": ["path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryReadTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let range = match (
            optional_usize(&arguments, "start_line")?,
            optional_usize(&arguments, "end_line")?,
        ) {
            (None, None) => None,
            (start, end) => Some(LineRange::new(start.unwrap_or(1), end)),
        };

        let memory = lock_memory(&self.state)?;
        let view = memory.view(&path, range).map_err(exec_err)?;
        Ok(memory_view_to_json(view))
    }
}

pub struct MemoryWriteTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryWriteTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_write".to_string(),
                description: "Create a new memory file under /memories.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Destination path under /memories." },
                        "content": { "type": "string", "description": "Full file content." }
                    },
                    "required": ["path", "content"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryWriteTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let content = required_string(&arguments, "content")?;
        let mut memory = lock_memory(&self.state)?;
        memory.create(&path, &content).map_err(exec_err)?;
        Ok(json!({ "ok": true, "path": path }))
    }
}

pub struct MemoryUpdateTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryUpdateTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_update".to_string(),
                description: "Replace a unique text fragment inside a memory file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "old_text": { "type": "string" },
                        "new_text": { "type": "string" }
                    },
                    "required": ["path", "old_text", "new_text"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryUpdateTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let old_text = required_string(&arguments, "old_text")?;
        let new_text = required_string(&arguments, "new_text")?;
        let mut memory = lock_memory(&self.state)?;
        memory
            .replace_text(&path, &old_text, &new_text)
            .map_err(exec_err)?;
        Ok(json!({ "ok": true, "path": path }))
    }
}

pub struct MemoryDeleteTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryDeleteTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_delete".to_string(),
                description: "Delete a file or directory under /memories.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryDeleteTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let mut memory = lock_memory(&self.state)?;
        memory.delete(&path).map_err(exec_err)?;
        Ok(json!({ "ok": true, "path": path }))
    }
}

pub struct MemoryRenameTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryRenameTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_rename".to_string(),
                description: "Rename or move a file or directory within /memories.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "old_path": { "type": "string" },
                        "new_path": { "type": "string" }
                    },
                    "required": ["old_path", "new_path"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryRenameTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let old_path = required_string(&arguments, "old_path")?;
        let new_path = required_string(&arguments, "new_path")?;
        let mut memory = lock_memory(&self.state)?;
        memory.rename(&old_path, &new_path).map_err(exec_err)?;
        Ok(json!({
            "ok": true,
            "old_path": old_path,
            "new_path": new_path
        }))
    }
}

pub struct MemoryInsertTool {
    definition: ToolDef,
    state: ToolState,
}

impl MemoryInsertTool {
    fn new(state: ToolState) -> Self {
        Self {
            definition: ToolDef {
                name: "memory_insert".to_string(),
                description: "Insert a line into a memory file at a zero-based insertion index."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "line": { "type": "integer", "minimum": 0, "description": "Zero-based insertion index." },
                        "text": { "type": "string" }
                    },
                    "required": ["path", "line", "text"]
                }),
            },
            state,
        }
    }
}

#[async_trait]
impl Tool for MemoryInsertTool {
    fn definition(&self) -> &ToolDef {
        &self.definition
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        let path = required_string(&arguments, "path")?;
        let line = required_usize(&arguments, "line")?;
        let text = required_string(&arguments, "text")?;
        let mut memory = lock_memory(&self.state)?;
        memory.insert(&path, line, &text).map_err(exec_err)?;
        Ok(json!({ "ok": true, "path": path, "line": line }))
    }
}

fn lock_memory(
    state: &ToolState,
) -> Result<std::sync::MutexGuard<'_, FsMemoryStore>, ToolExecutorError> {
    state
        .memory
        .lock()
        .map_err(|_| ToolExecutorError::ExecutionError("memory store lock poisoned".to_string()))
}

fn memory_view_to_json(view: MemoryView) -> Value {
    match view {
        MemoryView::Directory(listing) => json!({
            "kind": "directory",
            "path": listing.path,
            "entries": listing.entries,
        }),
        MemoryView::File(snapshot) => json!({
            "kind": "file",
            "path": snapshot.path,
            "lines": snapshot.lines,
        }),
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

fn required_usize(arguments: &Value, key: &str) -> Result<usize, ToolExecutorError> {
    optional_usize(arguments, key)?.ok_or_else(|| {
        ToolExecutorError::ExecutionError(format!("Missing or invalid '{key}' parameter"))
    })
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
