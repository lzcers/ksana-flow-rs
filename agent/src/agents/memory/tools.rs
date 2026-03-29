use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agents::memory::{
    FileSelector, FsMemoryStore, FsSelector, GrepRequest, LineRange, ListDirRequest, MemoryConfig,
    MemoryError, MemoryStore, MemoryView, ReadFileRequest, SelectConfig,
};
use crate::agents::{GenericToolExecutor, Tool, ToolDef, ToolExecutorError};

#[derive(Debug, Clone)]
pub struct MemoryToolConfig {
    pub memory: MemoryConfig,
    pub select: SelectConfig,
}

impl Default for MemoryToolConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
            select: SelectConfig::default(),
        }
    }
}

impl MemoryToolConfig {
    pub fn new(memory_base_path: impl Into<PathBuf>, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            memory: MemoryConfig::new(memory_base_path),
            select: SelectConfig::new(workspace_root),
        }
    }

    pub fn from_configs(memory: MemoryConfig, select: SelectConfig) -> Self {
        Self { memory, select }
    }
}

#[derive(Debug, Clone)]
struct ToolState {
    memory: Arc<Mutex<FsMemoryStore>>,
    selector: Arc<FsSelector>,
}

impl ToolState {
    fn new(config: MemoryToolConfig) -> Result<Self, MemoryError> {
        Ok(Self {
            memory: Arc::new(Mutex::new(FsMemoryStore::new(config.memory)?)),
            selector: Arc::new(FsSelector::new(config.select)),
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
    executor.register(MemoryInsertTool::new(state.clone()));
    executor.register(FileListTool::new(state.clone()));
    executor.register(FileSearchTool::new(state.clone()));
    executor.register(FileReadTool::new(state));
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

fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>, ToolExecutorError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|v| Some(v.to_string()))
            .ok_or_else(|| ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))),
    }
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

fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>, ToolExecutorError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| ToolExecutorError::ExecutionError(format!("Invalid '{key}' parameter"))),
    }
}
