use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use rusqlite::{Connection, params};
use serde_json::Value;

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
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let config = match get_config() {
            Ok(c) => c,
            Err(e) => return Ok(Value::String(format!("Error loading config: {}", e)).into()),
        };

        let conn = match Connection::open(&config.source.data_db_uri) {
            Ok(c) => c,
            Err(e) => return Ok(Value::String(format!("Error opening DB: {}", e)).into()),
        };

        let mut stmt = match conn.prepare("SELECT content FROM uploaded_files WHERE id = ?1") {
            Ok(s) => s,
            Err(e) => {
                return Ok(Value::String(format!("Error preparing statement: {}", e)).into());
            }
        };

        let content: Result<String, _> = stmt.query_row(params![self.file_id], |row| row.get(0));

        Ok(Value::String(content.expect("Error reading file content")).into())
    }
}
