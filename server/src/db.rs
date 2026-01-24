use anyhow::Result;
use flow::FlowEvent;
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
                workspace_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflow_executions (
                run_id TEXT PRIMARY KEY,
                workflow_id INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS execution_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                event TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS uploaded_files (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                size INTEGER NOT NULL,
                content TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                workspace_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Migration: Ensure workspace_id column exists and default data to 'ksana'
        Self::migrate_workspace_column(&conn, "workflows")?;
        Self::migrate_workspace_column(&conn, "uploaded_files")?;

        Ok(Self { conn })
    }

    fn migrate_workspace_column(conn: &Connection, table: &str) -> Result<()> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?;

        let mut has_workspace_id = false;
        for row in rows {
            if row? == "workspace_id" {
                has_workspace_id = true;
                break;
            }
        }

        if !has_workspace_id {
            conn.execute(
                &format!("ALTER TABLE {} ADD COLUMN workspace_id TEXT", table),
                [],
            )?;
            conn.execute(
                &format!(
                    "UPDATE {} SET workspace_id = 'ksana' WHERE workspace_id IS NULL",
                    table
                ),
                [],
            )?;
        }
        Ok(())
    }

    pub fn save_file(
        &self,
        id: &str,
        filename: &str,
        content: &str,
        size: i64,
        mime_type: &str,
        workspace_id: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO uploaded_files (id, filename, content, size, mime_type, workspace_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, filename, content, size, mime_type, workspace_id],
        )?;
        Ok(())
    }

    pub fn get_file(&self, id: &str, workspace_id: &str) -> Result<Option<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT filename, content FROM uploaded_files WHERE id = ?1 AND workspace_id = ?2")?;
        let mut rows = stmt.query(params![id, workspace_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }

    pub fn create_workflow(&self, name: &str, data: &Value, workspace_id: &str) -> Result<i64> {
        let data_json = serde_json::to_string(data)?;
        self.conn.execute(
            "INSERT INTO workflows (name, data, workspace_id) VALUES (?1, ?2, ?3)",
            params![name, data_json, workspace_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_workflow(&self, id: i64, workspace_id: &str) -> Result<Option<(String, Value)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, data FROM workflows WHERE id = ?1 AND workspace_id = ?2")?;
        let mut rows = stmt.query(params![id, workspace_id])?;

        if let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let data_str: String = row.get(1)?;
            let data: Value = serde_json::from_str(&data_str)?;
            Ok(Some((name, data)))
        } else {
            Ok(None)
        }
    }

    pub fn list_workflows(&self, workspace_id: &str) -> Result<Vec<(i64, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM workflows WHERE workspace_id = ?1 ORDER BY updated_at DESC")?;
        let rows = stmt.query_map(params![workspace_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(row?);
        }
        Ok(workflows)
    }

    pub fn update_workflow(&self, id: i64, name: &str, data: &Value, workspace_id: &str) -> Result<bool> {
        let data_json = serde_json::to_string(data)?;
        let count = self.conn.execute(
            "UPDATE workflows SET name = ?1, data = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND workspace_id = ?4",
            params![name, data_json, id, workspace_id],
        )?;
        Ok(count > 0)
    }

    pub fn delete_workflow(&self, id: i64, workspace_id: &str) -> Result<bool> {
        let count = self
            .conn
            .execute("DELETE FROM workflows WHERE id = ?1 AND workspace_id = ?2", params![id, workspace_id])?;
        Ok(count > 0)
    }

    pub fn create_execution(&self, run_id: &str, workflow_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workflow_executions (run_id, workflow_id, status) VALUES (?1, ?2, 'running')",
            params![run_id, workflow_id],
        )?;
        Ok(())
    }

    pub fn add_execution_event(&self, run_id: &str, event: &FlowEvent) -> Result<()> {
        let event_json = serde_json::to_string(&crate::utils::flow_event_to_value_lossy(event))?;
        self.conn.execute(
            "INSERT INTO execution_events (run_id, event) VALUES (?1, ?2)",
            params![run_id, event_json],
        )?;

        // Update status based on event
        let status = match event {
            FlowEvent::FlowFinished => Some("completed"),
            FlowEvent::FlowStopped => Some("stopped"),
            FlowEvent::FlowPaused => Some("paused"),
            FlowEvent::FlowResumed => Some("running"),
            FlowEvent::NodeError(node_id, _) if node_id == "runner" => Some("failed"),
            _ => None,
        };

        if let Some(s) = status {
            self.conn.execute(
                "UPDATE workflow_executions SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?2",
                params![s, run_id],
            )?;
        }

        Ok(())
    }

    pub fn get_latest_execution(
        &self,
        workflow_id: i64,
    ) -> Result<Option<(String, String, Vec<Value>)>> {
        // Get latest run_id
        let mut stmt = self.conn.prepare(
            "SELECT run_id, status FROM workflow_executions WHERE workflow_id = ?1 ORDER BY created_at DESC LIMIT 1"
        )?;
        let mut rows = stmt.query(params![workflow_id])?;

        if let Some(row) = rows.next()? {
            let run_id: String = row.get(0)?;
            let status: String = row.get(1)?;

            // Get events
            let mut event_stmt = self
                .conn
                .prepare("SELECT event FROM execution_events WHERE run_id = ?1 ORDER BY id ASC")?;
            let event_rows =
                event_stmt.query_map(params![run_id], |row| {
                    let event_str: String = row.get(0)?;
                    Ok(serde_json::from_str::<Value>(&event_str)
                        .unwrap_or(Value::String("FlowFinished".to_string())))
                })?;

            let mut events = Vec::new();
            for event in event_rows {
                if let Ok(e) = event {
                    events.push(e);
                }
            }

            Ok(Some((run_id, status, events)))
        } else {
            Ok(None)
        }
    }
}
