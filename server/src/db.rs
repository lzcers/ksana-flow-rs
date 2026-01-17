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

    pub fn create_execution(&self, run_id: &str, workflow_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workflow_executions (run_id, workflow_id, status) VALUES (?1, ?2, 'running')",
            params![run_id, workflow_id],
        )?;
        Ok(())
    }

    pub fn add_execution_event(&self, run_id: &str, event: &FlowEvent) -> Result<()> {
        let event_json = serde_json::to_string(event)?;
        self.conn.execute(
            "INSERT INTO execution_events (run_id, event) VALUES (?1, ?2)",
            params![run_id, event_json],
        )?;

        // Update status based on event
        let status = match event {
            FlowEvent::FlowFinished => Some("completed"),
            FlowEvent::FlowStopped => Some("stopped"),
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
    ) -> Result<Option<(String, String, Vec<FlowEvent>)>> {
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
                    Ok(serde_json::from_str::<FlowEvent>(&event_str)
                        .unwrap_or(FlowEvent::FlowFinished)) // Fallback or handle error better?
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
