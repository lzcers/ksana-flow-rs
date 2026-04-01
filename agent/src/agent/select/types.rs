use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file: String,
    pub line: usize,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub total_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDirRequest {
    pub path: PathBuf,
    pub max_depth: Option<usize>,
    pub pattern: Option<String>,
}

impl ListDirRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_depth: None,
            pattern: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindRequest {
    pub path: PathBuf,
    pub name_pattern: Option<String>,
    pub only_directories: bool,
    pub only_files: bool,
    pub max_depth: Option<usize>,
}

impl FindRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            name_pattern: None,
            only_directories: false,
            only_files: false,
            max_depth: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrepRequest {
    pub pattern: String,
    pub path: PathBuf,
    pub file_pattern: Option<String>,
    pub ignore_case: bool,
    pub max_results: Option<usize>,
}

impl GrepRequest {
    pub fn new(pattern: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            pattern: pattern.into(),
            path: path.into(),
            file_pattern: None,
            ignore_case: false,
            max_results: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub path: PathBuf,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

impl ReadFileRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            start_line: None,
            end_line: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectConfig {
    pub base_path: Option<PathBuf>,
    pub max_depth: usize,
    pub max_results: usize,
    pub max_file_size: u64,
    pub max_line_length: usize,
    pub max_read_lines: usize,
}

impl Default for SelectConfig {
    fn default() -> Self {
        Self {
            base_path: None,
            max_depth: 10,
            max_results: 100,
            max_file_size: 10 * 1024 * 1024,
            max_line_length: 2_000,
            max_read_lines: 1_000,
        }
    }
}

impl SelectConfig {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: Some(base_path.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SelectError {
    #[error("path traversal is not allowed: {0}")]
    PathTraversal(String),
    #[error("path is outside allowed base path: {0}")]
    OutsideBasePath(String),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("path is not a directory: {0}")]
    NotDirectory(String),
    #[error("path is a directory, not a file: {0}")]
    NotFile(String),
    #[error("regex error: {0}")]
    Regex(String),
    #[error("file too large: {actual} bytes > {limit} bytes")]
    FileTooLarge { actual: u64, limit: u64 },
    #[error("i/o error: {0}")]
    Io(String),
}
