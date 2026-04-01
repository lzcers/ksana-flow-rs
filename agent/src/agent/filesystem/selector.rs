use std::fs;
use std::path::{Component, Path, PathBuf};

use glob::Pattern;
use regex::RegexBuilder;
use walkdir::WalkDir;

use crate::agent::select::{
    FileContent, FileEntry, FileSelector, FindRequest, GrepMatch, GrepRequest, ListDirRequest,
    ReadFileRequest, SelectConfig, SelectError,
};

/// 基于文件系统的 Select 实现。
#[derive(Debug, Clone)]
pub struct FsSelector {
    config: SelectConfig,
}

impl FsSelector {
    pub fn new(config: SelectConfig) -> Self {
        Self { config }
    }

    fn resolve_path(&self, path: &Path) -> Result<PathBuf, SelectError> {
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    return Err(SelectError::PathTraversal(path.display().to_string()));
                }
                Component::RootDir
                | Component::Prefix(_)
                | Component::Normal(_)
                | Component::CurDir => {}
            }
        }

        let base = self
            .config
            .base_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        };
        let resolved = candidate
            .canonicalize()
            .map_err(|_| SelectError::NotFound(path.display().to_string()))?;

        if let Some(base_path) = &self.config.base_path {
            let base_resolved = base_path
                .canonicalize()
                .map_err(|e| SelectError::Io(e.to_string()))?;
            if !resolved.starts_with(&base_resolved) {
                return Err(SelectError::OutsideBasePath(path.display().to_string()));
            }
        }

        Ok(resolved)
    }

    fn root_display<'a>(&self, request_path: &'a Path) -> &'a Path {
        if request_path.as_os_str().is_empty() {
            Path::new(".")
        } else {
            request_path
        }
    }

    fn relative_display(&self, root: &Path, path: &Path) -> String {
        path.strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}

impl FileSelector for FsSelector {
    fn config(&self) -> &SelectConfig {
        &self.config
    }

    fn list_dir(&self, request: &ListDirRequest) -> Result<Vec<FileEntry>, SelectError> {
        let root = self.resolve_path(&request.path)?;
        if !root.is_dir() {
            return Err(SelectError::NotDirectory(
                self.root_display(&request.path).display().to_string(),
            ));
        }

        let max_depth = request.max_depth.unwrap_or(self.config.max_depth);
        let pattern = request
            .pattern
            .as_deref()
            .map(Pattern::new)
            .transpose()
            .map_err(|e| SelectError::Io(e.to_string()))?;

        let mut entries = Vec::new();
        for entry in WalkDir::new(&root)
            .min_depth(1)
            .max_depth(max_depth)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|e| SelectError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if let Some(pattern) = &pattern {
                if !pattern.matches(&name) && !entry.file_type().is_dir() {
                    continue;
                }
            }
            let metadata = entry
                .metadata()
                .map_err(|e| SelectError::Io(e.to_string()))?;
            entries.push(FileEntry {
                path: self.relative_display(&root, entry.path()),
                is_dir: metadata.is_dir(),
                size_bytes: metadata.is_file().then_some(metadata.len()),
            });
        }

        Ok(entries)
    }

    fn find(&self, request: &FindRequest) -> Result<Vec<String>, SelectError> {
        let root = self.resolve_path(&request.path)?;
        if !root.is_dir() {
            return Err(SelectError::NotDirectory(
                self.root_display(&request.path).display().to_string(),
            ));
        }

        let max_depth = request.max_depth.unwrap_or(self.config.max_depth);
        let name_pattern = request
            .name_pattern
            .as_deref()
            .map(Pattern::new)
            .transpose()
            .map_err(|e| SelectError::Io(e.to_string()))?;

        let mut results = Vec::new();
        for entry in WalkDir::new(&root)
            .min_depth(1)
            .max_depth(max_depth)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|e| SelectError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') {
                continue;
            }

            let metadata = entry
                .metadata()
                .map_err(|e| SelectError::Io(e.to_string()))?;
            if request.only_files && metadata.is_dir() {
                continue;
            }
            if request.only_directories && metadata.is_file() {
                continue;
            }
            if let Some(pattern) = &name_pattern {
                if !pattern.matches(&name) {
                    continue;
                }
            }

            results.push(self.relative_display(&root, entry.path()));
        }

        Ok(results)
    }

    fn grep(&self, request: &GrepRequest) -> Result<Vec<GrepMatch>, SelectError> {
        let root = self.resolve_path(&request.path)?;
        let regex = RegexBuilder::new(&request.pattern)
            .case_insensitive(request.ignore_case)
            .build()
            .map_err(|e| SelectError::Regex(e.to_string()))?;
        let file_pattern = request
            .file_pattern
            .as_deref()
            .map(Pattern::new)
            .transpose()
            .map_err(|e| SelectError::Io(e.to_string()))?;
        let max_results = request.max_results.unwrap_or(self.config.max_results);

        let mut matches = Vec::new();
        if root.is_file() {
            search_file(
                &root,
                &root,
                &regex,
                max_results,
                self.config.max_file_size,
                self.config.max_line_length,
                &mut matches,
            )?;
            return Ok(matches);
        }

        if !root.is_dir() {
            return Err(SelectError::NotDirectory(
                self.root_display(&request.path).display().to_string(),
            ));
        }

        for entry in WalkDir::new(&root).sort_by_file_name() {
            let entry = entry.map_err(|e| SelectError::Io(e.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if let Some(pattern) = &file_pattern {
                if !pattern.matches(&name) {
                    continue;
                }
            }
            search_file(
                &root,
                entry.path(),
                &regex,
                max_results,
                self.config.max_file_size,
                self.config.max_line_length,
                &mut matches,
            )?;
            if matches.len() >= max_results {
                break;
            }
        }

        Ok(matches)
    }

    fn read_file(&self, request: &ReadFileRequest) -> Result<FileContent, SelectError> {
        let path = self.resolve_path(&request.path)?;
        if !path.exists() {
            return Err(SelectError::NotFound(
                self.root_display(&request.path).display().to_string(),
            ));
        }
        if path.is_dir() {
            return Err(SelectError::NotFile(
                self.root_display(&request.path).display().to_string(),
            ));
        }

        let metadata = fs::metadata(&path).map_err(|e| SelectError::Io(e.to_string()))?;
        if metadata.len() > self.config.max_file_size {
            return Err(SelectError::FileTooLarge {
                actual: metadata.len(),
                limit: self.config.max_file_size,
            });
        }

        let content = fs::read_to_string(&path).map_err(|e| SelectError::Io(e.to_string()))?;
        let lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        let total_lines = lines.len();
        let start = request.start_line.unwrap_or(1).max(1);
        let mut end = request.end_line.unwrap_or(total_lines).min(total_lines);
        if end < start {
            end = start;
        }
        if end.saturating_sub(start).saturating_add(1) > self.config.max_read_lines {
            end = start + self.config.max_read_lines - 1;
        }

        let selected = lines
            .iter()
            .skip(start.saturating_sub(1))
            .take(end.saturating_sub(start).saturating_add(1))
            .map(|line| truncate_line(line, self.config.max_line_length))
            .collect::<Vec<_>>();

        Ok(FileContent {
            path: request.path.display().to_string(),
            content: selected.join("\n"),
            start_line: start,
            end_line: selected
                .len()
                .checked_sub(1)
                .map(|offset| start + offset)
                .unwrap_or(start),
            total_lines,
        })
    }
}

fn search_file(
    root: &Path,
    path: &Path,
    regex: &regex::Regex,
    max_results: usize,
    max_file_size: u64,
    max_line_length: usize,
    matches: &mut Vec<GrepMatch>,
) -> Result<(), SelectError> {
    let metadata = fs::metadata(path).map_err(|e| SelectError::Io(e.to_string()))?;
    if metadata.len() > max_file_size {
        return Ok(());
    }
    let content = fs::read_to_string(path).map_err(|e| SelectError::Io(e.to_string()))?;
    for (index, line) in content.lines().enumerate() {
        if matches.len() >= max_results {
            break;
        }
        if regex.is_match(line) {
            matches.push(GrepMatch {
                file: path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
                line: index + 1,
                content: truncate_line(line, max_line_length),
            });
        }
    }
    Ok(())
}

fn truncate_line(line: &str, max_line_length: usize) -> String {
    if line.chars().count() <= max_line_length {
        line.to_string()
    } else {
        format!(
            "{}...",
            line.chars().take(max_line_length).collect::<String>()
        )
    }
}
