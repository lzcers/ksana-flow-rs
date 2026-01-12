use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn create_workflow(&self, name: &str, data: &Value) -> Result<i64> {
        let data_json = serde_json::to_string(data)?;
        self.conn.execute(
            "INSERT INTO workflows (name, data) VALUES (?1, ?2)",
            params![name, data_json],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_workflow(&self, id: i64) -> Result<Option<(String, Value)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, data FROM workflows WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let data_str: String = row.get(1)?;
            let data: Value = serde_json::from_str(&data_str)?;
            Ok(Some((name, data)))
        } else {
            Ok(None)
        }
    }

    pub fn list_workflows(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM workflows ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(row?);
        }
        Ok(workflows)
    }

    pub fn update_workflow(&self, id: i64, name: &str, data: &Value) -> Result<bool> {
        let data_json = serde_json::to_string(data)?;
        let count = self.conn.execute(
            "UPDATE workflows SET name = ?1, data = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![name, data_json, id],
        )?;
        Ok(count > 0)
    }

    pub fn delete_workflow(&self, id: i64) -> Result<bool> {
        let count = self
            .conn
            .execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}
