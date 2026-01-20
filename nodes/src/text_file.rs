use async_trait::async_trait;
use flow::{Context, Node};
use rusqlite::{Connection, params};

use crate::config::get_config;

pub struct TextFileNode {
    file_id: String,
}

impl TextFileNode {
    pub fn new(file_id: String) -> Self {
        Self { file_id }
    }
}

#[async_trait]
impl Node for TextFileNode {
    type In = ();
    type Out = String;

    async fn run(&mut self, _ctx: &Context, _input: Self::In) -> Self::Out {
        let config = match get_config() {
            Ok(c) => c,
            Err(e) => return format!("Error loading config: {}", e),
        };

        let conn = match Connection::open(&config.source.data_db_uri) {
            Ok(c) => c,
            Err(e) => return format!("Error opening DB: {}", e),
        };

        let mut stmt = match conn.prepare("SELECT content FROM uploaded_files WHERE id = ?1") {
            Ok(s) => s,
            Err(e) => return format!("Error preparing statement: {}", e),
        };

        let content: Result<String, _> = stmt.query_row(params![self.file_id], |row| row.get(0));

        content.expect("Error reading file content")
    }
}
