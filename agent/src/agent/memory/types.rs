use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 行范围，使用 1-based 的闭区间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub start: usize,
    pub end: Option<usize>,
}

impl LineRange {
    pub fn new(start: usize, end: Option<usize>) -> Self {
        Self { start, end }
    }
}

/// Memory 子系统的安全与资源配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub base_path: PathBuf,
    pub max_file_size: usize,
    pub max_line_length: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./memory"),
            max_file_size: 1024 * 1024,
            max_line_length: 2_000,
        }
    }
}

impl MemoryConfig {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Self::default()
        }
    }

    pub fn memory_root(&self) -> PathBuf {
        self.base_path.join("memories")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub path: String,
    pub size_bytes: usize,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryListing {
    pub path: String,
    pub entries: Vec<MemoryEntry>,
}

impl Display for DirectoryListing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Here're the files and directories up to 2 levels deep in {}, excluding hidden items and node_modules:",
            self.path
        )?;
        for entry in &self.entries {
            writeln!(
                f,
                "{}\t{}",
                human_readable_size(entry.size_bytes),
                entry.path
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    pub lines: Vec<(usize, String)>,
}

impl Display for FileSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Here's the content of {} with line numbers:", self.path)?;
        for (line, content) in &self.lines {
            writeln!(f, "{line:6}\t{content}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryView {
    Directory(DirectoryListing),
    File(FileSnapshot),
}

impl Display for MemoryView {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Directory(listing) => listing.fmt(f),
            Self::File(snapshot) => snapshot.fmt(f),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MemoryError {
    #[error("path must start with /memories: {0}")]
    InvalidPath(String),
    #[error("path escapes /memories: {0}")]
    PathTraversal(String),
    #[error("path not found: {0}")]
    PathNotFound(String),
    #[error("cannot delete /memories root")]
    CannotDeleteRoot,
    #[error("file already exists: {0}")]
    FileExists(String),
    #[error("destination already exists: {0}")]
    DestinationExists(String),
    #[error("content exceeds max size {max_bytes} bytes, got {actual_bytes} bytes")]
    ContentTooLarge {
        max_bytes: usize,
        actual_bytes: usize,
    },
    #[error("text not found in file: {0}")]
    TextNotFound(String),
    #[error("multiple occurrences found for `{text}` at lines {lines:?}")]
    MultipleOccurrences { text: String, lines: Vec<usize> },
    #[error("invalid insert line {line}, valid range is [0, {max_lines}]")]
    InvalidLine { line: usize, max_lines: usize },
    #[error("i/o error: {0}")]
    Io(String),
}

fn human_readable_size(size_bytes: usize) -> String {
    match size_bytes {
        0..=1023 => format!("{size_bytes}B"),
        1024..=1_048_575 => format!("{:.1}K", size_bytes as f64 / 1024.0),
        1_048_576..=1_073_741_823 => format!("{:.1}M", size_bytes as f64 / 1_048_576.0),
        _ => format!("{:.1}G", size_bytes as f64 / 1_073_741_824.0),
    }
}
