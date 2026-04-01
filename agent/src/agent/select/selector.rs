use crate::agent::select::ListDirRequest;
use crate::agent::select::{
    FileContent, FileEntry, FindRequest, GrepMatch, GrepRequest, ReadFileRequest, SelectConfig,
    SelectError,
};

/// 文件探索端口。
pub trait FileSelector {
    fn config(&self) -> &SelectConfig;
    fn list_dir(&self, request: &ListDirRequest) -> Result<Vec<FileEntry>, SelectError>;
    fn find(&self, request: &FindRequest) -> Result<Vec<String>, SelectError>;
    fn grep(&self, request: &GrepRequest) -> Result<Vec<GrepMatch>, SelectError>;
    fn read_file(&self, request: &ReadFileRequest) -> Result<FileContent, SelectError>;
}
