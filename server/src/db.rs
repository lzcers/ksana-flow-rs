use anyhow::Result;
use flow::{FlowEvent, FlowEventEnvelope, RunnerKind};
use rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

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

        Self::migrate_execution_events_table(&conn)?;

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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_media (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                data BLOB NOT NULL,
                prompt TEXT NOT NULL,
                model TEXT NOT NULL,
                workspace_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Migration: Ensure workspace_id column exists and default data to 'ksana'
        Self::migrate_workspace_column(&conn, "workflows")?;
        Self::migrate_workspace_column(&conn, "uploaded_files")?;
        Self::migrate_uploaded_files_blob_column(&conn)?;

        Ok(Self { conn })
    }

    fn migrate_execution_events_table(conn: &Connection) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(execution_events)")?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?;

        let mut has_runner_id = false;
        let mut has_event_type = false;
        for row in rows {
            match row?.as_str() {
                "runner_id" => has_runner_id = true,
                "event_type" => has_event_type = true,
                _ => {}
            }
        }

        if !(has_runner_id && has_event_type) {
            conn.execute("DROP TABLE IF EXISTS execution_events", [])?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS execution_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    run_id TEXT NOT NULL,
                    runner_id INTEGER NOT NULL,
                    runner_kind TEXT NOT NULL,
                    parent_runner_id INTEGER,
                    parent_node_id TEXT,
                    event_type TEXT NOT NULL,
                    node_id TEXT,
                    payload_json TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_execution_events_run_id_id ON execution_events(run_id, id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_execution_events_run_id_runner_id ON execution_events(run_id, runner_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_execution_events_run_id_parent_node_id ON execution_events(run_id, parent_node_id)",
                [],
            )?;
        }

        Ok(())
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

    fn migrate_uploaded_files_blob_column(conn: &Connection) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(uploaded_files)")?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?;

        let mut has_content_blob = false;
        for row in rows {
            if row? == "content_blob" {
                has_content_blob = true;
                break;
            }
        }

        if !has_content_blob {
            conn.execute(
                "ALTER TABLE uploaded_files ADD COLUMN content_blob BLOB",
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
        let mut stmt = self.conn.prepare(
            "SELECT filename, content FROM uploaded_files WHERE id = ?1 AND workspace_id = ?2",
        )?;
        let mut rows = stmt.query(params![id, workspace_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }

    pub fn save_file_bytes(
        &self,
        id: &str,
        filename: &str,
        content_bytes: &[u8],
        size: i64,
        mime_type: &str,
        workspace_id: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO uploaded_files (id, filename, content, content_blob, size, mime_type, workspace_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, filename, "", content_bytes, size, mime_type, workspace_id],
        )?;
        Ok(())
    }

    pub fn get_file_data(
        &self,
        id: &str,
        workspace_id: &str,
    ) -> Result<Option<(String, String, String, Option<Vec<u8>>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT filename, mime_type, content, content_blob FROM uploaded_files WHERE id = ?1 AND workspace_id = ?2",
        )?;
        let mut rows = stmt.query(params![id, workspace_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        } else {
            Ok(None)
        }
    }

    pub fn save_ai_media(
        &self,
        id: &str,
        kind: &str,
        mime_type: &str,
        data: &[u8],
        size: i64,
        prompt: &str,
        model: &str,
        workspace_id: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ai_media (id, kind, mime_type, size, data, prompt, model, workspace_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, kind, mime_type, size, data, prompt, model, workspace_id],
        )?;
        Ok(())
    }

    pub fn get_ai_media(&self, id: &str, workspace_id: &str) -> Result<Option<(String, Vec<u8>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT mime_type, data FROM ai_media WHERE id = ?1 AND workspace_id = ?2")?;
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
        let mut stmt = self.conn.prepare(
            "SELECT id, name FROM workflows WHERE workspace_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(row?);
        }
        Ok(workflows)
    }

    pub fn update_workflow(
        &self,
        id: i64,
        name: &str,
        data: &Value,
        workspace_id: &str,
    ) -> Result<bool> {
        let data_json = serde_json::to_string(data)?;
        let count = self.conn.execute(
            "UPDATE workflows SET name = ?1, data = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND workspace_id = ?4",
            params![name, data_json, id, workspace_id],
        )?;
        Ok(count > 0)
    }

    pub fn delete_workflow(&self, id: i64, workspace_id: &str) -> Result<bool> {
        let count = self.conn.execute(
            "DELETE FROM workflows WHERE id = ?1 AND workspace_id = ?2",
            params![id, workspace_id],
        )?;
        Ok(count > 0)
    }

    pub fn create_execution(&self, run_id: &str, workflow_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workflow_executions (run_id, workflow_id, status) VALUES (?1, ?2, 'running')",
            params![run_id, workflow_id],
        )?;
        Ok(())
    }

    pub fn add_execution_event(&self, run_id: &str, envelope: &FlowEventEnvelope) -> Result<()> {
        if matches!(envelope.event, FlowEvent::NodeStreamNextMessage(_, _)) {
            return Ok(());
        }

        let (event_type, node_id, payload_json) = match &envelope.event {
            FlowEvent::NodeStarted(node_id) => ("NodeStarted", Some(node_id.clone()), None),
            FlowEvent::NodeCompleted(node_id) => ("NodeCompleted", Some(node_id.clone()), None),
            FlowEvent::NodeStreamStarted(node_id) => {
                ("NodeStreamStarted", Some(node_id.clone()), None)
            }
            FlowEvent::NodeError(node_id, msg) => (
                "NodeError",
                Some(node_id.clone()),
                Some(serde_json::to_string(&Value::String(msg.clone()))?),
            ),
            FlowEvent::NodeInMessage(node_id, inputs) => (
                "NodeInMessage",
                Some(node_id.clone()),
                Some(serde_json::to_string(
                    &crate::utils::node_inputs_to_value_lossy(inputs),
                )?),
            ),
            FlowEvent::NodeOutMessage(node_id, payload) => (
                "NodeOutMessage",
                Some(node_id.clone()),
                Some(serde_json::to_string(payload)?),
            ),
            FlowEvent::NodeStreamNextMessage(_, _) => unreachable!(),
            FlowEvent::FlowStarted => ("FlowStarted", None, None),
            FlowEvent::FlowPaused => ("FlowPaused", None, None),
            FlowEvent::FlowResumed => ("FlowResumed", None, None),
            FlowEvent::FlowStopped => ("FlowStopped", None, None),
            FlowEvent::FlowFinished => ("FlowFinished", None, None),
        };

        let runner_kind = match envelope.runner_kind {
            RunnerKind::Root => "Root",
            RunnerKind::Subgraph => "Subgraph",
        };

        self.conn.execute(
            "INSERT INTO execution_events (
                run_id,
                runner_id,
                runner_kind,
                parent_runner_id,
                parent_node_id,
                event_type,
                node_id,
                payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run_id,
                envelope.runner_id,
                runner_kind,
                envelope.parent_runner_id,
                envelope.parent_node_id,
                event_type,
                node_id,
                payload_json
            ],
        )?;

        // Update status based on event
        let status = match &envelope.event {
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
            let mut event_stmt = self.conn.prepare(
                "SELECT
                    runner_id,
                    runner_kind,
                    parent_runner_id,
                    parent_node_id,
                    event_type,
                    node_id,
                    payload_json,
                    created_at
                 FROM execution_events
                 WHERE run_id = ?1
                 ORDER BY id ASC",
            )?;
            let event_rows = event_stmt.query_map(params![run_id], |row| {
                let runner_id: i64 = row.get(0)?;
                let runner_kind: String = row.get(1)?;
                let parent_runner_id: Option<i64> = row.get(2)?;
                let parent_node_id: Option<String> = row.get(3)?;
                let event_type: String = row.get(4)?;
                let node_id: Option<String> = row.get(5)?;
                let payload_json: Option<String> = row.get(6)?;
                let created_at: String = row.get(7)?;

                let mut event_obj = Map::new();
                event_obj.insert("type".to_string(), Value::String(event_type));
                if let Some(node_id) = node_id {
                    event_obj.insert("nodeId".to_string(), Value::String(node_id));
                }
                if let Some(payload_json) = payload_json {
                    let msg = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
                    event_obj.insert("msg".to_string(), msg);
                }

                Ok(json!({
                    "runnerId": runner_id as u64,
                    "runnerKind": runner_kind,
                    "parentRunnerId": parent_runner_id.map(|v| v as u64),
                    "parentNodeId": parent_node_id,
                    "createdAt": created_at,
                    "event": Value::Object(event_obj),
                }))
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
