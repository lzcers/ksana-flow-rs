use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::agent::memory::{
    DirectoryListing, FileSnapshot, LineRange, MemoryConfig, MemoryEntry, MemoryError, MemoryStore,
    MemoryView,
};

/// 基于文件系统的 Memory 实现。
#[derive(Debug, Clone)]
pub struct FsMemoryStore {
    config: MemoryConfig,
}

impl FsMemoryStore {
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        fs::create_dir_all(config.memory_root()).map_err(io_error)?;
        Ok(Self { config })
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, MemoryError> {
        if !path.starts_with("/memories") {
            return Err(MemoryError::InvalidPath(path.to_string()));
        }

        let relative = path.trim_start_matches("/memories").trim_start_matches('/');
        let relative_path = Path::new(relative);
        for component in relative_path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => {
                    return Err(MemoryError::PathTraversal(path.to_string()));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(MemoryError::PathTraversal(path.to_string()));
                }
            }
        }

        Ok(if relative.is_empty() {
            self.config.memory_root()
        } else {
            self.config.memory_root().join(relative)
        })
    }

    fn dir_size(path: &Path) -> usize {
        let mut total = 0usize;
        let Ok(entries) = fs::read_dir(path) else {
            return total;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total += metadata.len() as usize;
                } else if metadata.is_dir() {
                    total += Self::dir_size(&path);
                }
            }
        }
        total
    }

    fn list_directory(
        &self,
        path: &str,
        full_path: &Path,
    ) -> Result<DirectoryListing, MemoryError> {
        let mut entries = Vec::new();
        entries.push(MemoryEntry {
            path: path.to_string(),
            size_bytes: Self::dir_size(full_path),
            is_dir: true,
        });

        let items = fs::read_dir(full_path).map_err(io_error)?;
        for item in items.flatten() {
            let file_name = item.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with('.') || file_name == "node_modules" {
                continue;
            }

            let item_path = item.path();
            let display_path = if path == "/memories" {
                format!("/memories/{file_name}")
            } else {
                format!("{path}/{file_name}")
            };

            let metadata = item.metadata().map_err(io_error)?;
            if metadata.is_file() {
                entries.push(MemoryEntry {
                    path: display_path,
                    size_bytes: metadata.len() as usize,
                    is_dir: false,
                });
            } else if metadata.is_dir() {
                entries.push(MemoryEntry {
                    path: display_path.clone(),
                    size_bytes: Self::dir_size(&item_path),
                    is_dir: true,
                });

                let nested = fs::read_dir(&item_path).map_err(io_error)?;
                for subitem in nested.flatten() {
                    let nested_name = subitem.file_name();
                    let nested_name = nested_name.to_string_lossy();
                    if nested_name.starts_with('.') || nested_name == "node_modules" {
                        continue;
                    }
                    let nested_path = item_path.join(nested_name.as_ref());
                    let nested_metadata = subitem.metadata().map_err(io_error)?;
                    entries.push(MemoryEntry {
                        path: format!("{display_path}/{nested_name}"),
                        size_bytes: if nested_metadata.is_file() {
                            nested_metadata.len() as usize
                        } else {
                            Self::dir_size(&nested_path)
                        },
                        is_dir: nested_metadata.is_dir(),
                    });
                }
            }
        }

        Ok(DirectoryListing {
            path: path.to_string(),
            entries,
        })
    }

    fn read_file_view(
        &self,
        path: &str,
        full_path: &Path,
        range: Option<LineRange>,
    ) -> Result<FileSnapshot, MemoryError> {
        let content = fs::read_to_string(full_path).map_err(io_error)?;
        let all_lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        let total_lines = all_lines.len();
        let start = range.map(|it| it.start.max(1)).unwrap_or(1);
        let end = range
            .and_then(|it| it.end)
            .unwrap_or(total_lines)
            .min(total_lines);

        let mut lines = Vec::new();
        for (offset, line) in all_lines
            .iter()
            .enumerate()
            .skip(start.saturating_sub(1))
            .take(end.saturating_sub(start).saturating_add(1))
        {
            lines.push((offset + 1, truncate_line(line, self.config.max_line_length)));
        }

        Ok(FileSnapshot {
            path: path.to_string(),
            lines,
        })
    }

    fn read_file(&self, path: &str) -> Result<String, MemoryError> {
        let full_path = self.validate_path(path)?;
        if !full_path.exists() || full_path.is_dir() {
            return Err(MemoryError::PathNotFound(path.to_string()));
        }
        fs::read_to_string(full_path).map_err(io_error)
    }
}

impl MemoryStore for FsMemoryStore {
    fn config(&self) -> &MemoryConfig {
        &self.config
    }

    fn view(&self, path: &str, range: Option<LineRange>) -> Result<MemoryView, MemoryError> {
        let full_path = self.validate_path(path)?;
        if !full_path.exists() {
            return Err(MemoryError::PathNotFound(path.to_string()));
        }

        if full_path.is_dir() {
            Ok(MemoryView::Directory(
                self.list_directory(path, &full_path)?,
            ))
        } else {
            Ok(MemoryView::File(
                self.read_file_view(path, &full_path, range)?,
            ))
        }
    }

    fn create(&mut self, path: &str, content: &str) -> Result<(), MemoryError> {
        let full_path = self.validate_path(path)?;
        if full_path.exists() {
            return Err(MemoryError::FileExists(path.to_string()));
        }
        let size = content.len();
        if size > self.config.max_file_size {
            return Err(MemoryError::ContentTooLarge {
                max_bytes: self.config.max_file_size,
                actual_bytes: size,
            });
        }
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::write(full_path, content).map_err(io_error)
    }

    fn replace_text(
        &mut self,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<(), MemoryError> {
        let full_path = self.validate_path(path)?;
        if !full_path.exists() || full_path.is_dir() {
            return Err(MemoryError::PathNotFound(path.to_string()));
        }

        let content = fs::read_to_string(&full_path).map_err(io_error)?;
        let matches: Vec<usize> = find_line_numbers(&content, old_text);
        match matches.len() {
            0 => return Err(MemoryError::TextNotFound(old_text.to_string())),
            1 => {}
            _ => {
                return Err(MemoryError::MultipleOccurrences {
                    text: old_text.to_string(),
                    lines: matches,
                });
            }
        }

        let replaced = content.replacen(old_text, new_text, 1);
        fs::write(full_path, replaced).map_err(io_error)
    }

    fn insert(&mut self, path: &str, line: usize, text: &str) -> Result<(), MemoryError> {
        let full_path = self.validate_path(path)?;
        if !full_path.exists() || full_path.is_dir() {
            return Err(MemoryError::PathNotFound(path.to_string()));
        }

        let content = self.read_file(path)?;
        let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        let max_lines = lines.len();
        if line > max_lines {
            return Err(MemoryError::InvalidLine { line, max_lines });
        }

        lines.insert(line, text.trim_end_matches('\n').to_string());
        let next = if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        };
        fs::write(full_path, next).map_err(io_error)
    }

    fn delete(&mut self, path: &str) -> Result<(), MemoryError> {
        if path == "/memories" {
            return Err(MemoryError::CannotDeleteRoot);
        }
        let full_path = self.validate_path(path)?;
        if !full_path.exists() {
            return Err(MemoryError::PathNotFound(path.to_string()));
        }
        if full_path.is_dir() {
            fs::remove_dir_all(full_path).map_err(io_error)
        } else {
            fs::remove_file(full_path).map_err(io_error)
        }
    }

    fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), MemoryError> {
        let old_full_path = self.validate_path(old_path)?;
        let new_full_path = self.validate_path(new_path)?;
        if !old_full_path.exists() {
            return Err(MemoryError::PathNotFound(old_path.to_string()));
        }
        if new_full_path.exists() {
            return Err(MemoryError::DestinationExists(new_path.to_string()));
        }
        if let Some(parent) = new_full_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::rename(old_full_path, new_full_path).map_err(io_error)
    }

    fn clear_all(&mut self) -> Result<(), MemoryError> {
        let root = self.config.memory_root();
        if root.exists() {
            fs::remove_dir_all(&root).map_err(io_error)?;
        }
        fs::create_dir_all(root).map_err(io_error)
    }
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

fn find_line_numbers(content: &str, search: &str) -> Vec<usize> {
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| line.contains(search).then_some(index + 1))
        .collect()
}

fn io_error(error: std::io::Error) -> MemoryError {
    MemoryError::Io(error.to_string())
}
