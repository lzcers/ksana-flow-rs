use crate::agent::memory::{LineRange, MemoryConfig, MemoryError, MemoryView};

/// 持久化记忆端口。
pub trait MemoryStore {
    fn config(&self) -> &MemoryConfig;
    fn view(&self, path: &str, range: Option<LineRange>) -> Result<MemoryView, MemoryError>;
    fn create(&mut self, path: &str, content: &str) -> Result<(), MemoryError>;
    fn replace_text(
        &mut self,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<(), MemoryError>;
    fn insert(&mut self, path: &str, line: usize, text: &str) -> Result<(), MemoryError>;
    fn delete(&mut self, path: &str) -> Result<(), MemoryError>;
    fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), MemoryError>;
    fn clear_all(&mut self) -> Result<(), MemoryError>;
}
