use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowContext, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::api::archive::{SaveSlotRecord, SessionArchivePayload};

const CREATE_SAVE_SLOTS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS save_slots (
    slot_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    title TEXT NOT NULL,
    turn_index INTEGER NOT NULL,
    phase TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archive_json TEXT NOT NULL
);
"#;

const CREATE_SAVE_SLOTS_SESSION_INDEX_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_save_slots_session_id
ON save_slots(session_id);
"#;

const CREATE_SAVE_SLOTS_UPDATED_INDEX_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_save_slots_updated_at
ON save_slots(updated_at DESC);
"#;

#[derive(Debug, Clone)]
pub struct ArchiveRepository {
    db_path: PathBuf,
}

impl ArchiveRepository {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(CREATE_SAVE_SLOTS_TABLE_SQL)
            .context("failed to create save_slots table")?;
        conn.execute_batch(CREATE_SAVE_SLOTS_SESSION_INDEX_SQL)
            .context("failed to create save_slots session index")?;
        conn.execute_batch(CREATE_SAVE_SLOTS_UPDATED_INDEX_SQL)
            .context("failed to create save_slots updated_at index")?;
        Ok(())
    }

    pub fn upsert_save_slot(
        &self,
        slot_id: &str,
        payload: &SessionArchivePayload,
    ) -> Result<SaveSlotRecord> {
        let conn = self.open_connection()?;
        let archive_json =
            serde_json::to_string(payload).context("failed to serialize SessionArchivePayload")?;
        let turn_index =
            i64::try_from(payload.turn_state.turn_index).context("turn_index exceeds sqlite INTEGER range")?;
        let phase = turn_phase_storage_value(payload)?;
        let now = now_string();
        let created_at = self
            .existing_created_at(&conn, slot_id)?
            .unwrap_or_else(|| now.clone());

        conn.execute(
            r#"
            INSERT INTO save_slots (
                slot_id,
                session_id,
                title,
                turn_index,
                phase,
                created_at,
                updated_at,
                archive_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(slot_id) DO UPDATE SET
                session_id = excluded.session_id,
                title = excluded.title,
                turn_index = excluded.turn_index,
                phase = excluded.phase,
                updated_at = excluded.updated_at,
                archive_json = excluded.archive_json
            "#,
            params![
                slot_id,
                payload.session_id,
                payload.title,
                turn_index,
                phase,
                created_at,
                now,
                archive_json
            ],
        )
        .with_context(|| format!("failed to upsert save slot `{slot_id}`"))?;

        self.load_save_slot_record(slot_id)?
            .with_context(|| format!("save slot `{slot_id}` was written but could not be read back"))
    }

    pub fn load_archive_payload(&self, slot_id: &str) -> Result<Option<SessionArchivePayload>> {
        let conn = self.open_connection()?;
        let archive_json: Option<String> = conn
            .query_row(
                "SELECT archive_json FROM save_slots WHERE slot_id = ?1",
                [slot_id],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| format!("failed to query archive_json for slot `{slot_id}`"))?;

        archive_json
            .map(|raw| {
                serde_json::from_str(&raw)
                    .with_context(|| format!("failed to deserialize SessionArchivePayload for slot `{slot_id}`"))
            })
            .transpose()
    }

    pub fn load_save_slot_record(&self, slot_id: &str) -> Result<Option<SaveSlotRecord>> {
        let conn = self.open_connection()?;
        let row: Option<(String, String, String, String, String, String)> = conn
            .query_row(
                r#"
                SELECT slot_id, session_id, title, created_at, updated_at, archive_json
                FROM save_slots
                WHERE slot_id = ?1
                "#,
                [slot_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .with_context(|| format!("failed to load save slot `{slot_id}`"))?;

        row.map(
            |(slot_id, session_id, title, created_at, updated_at, archive_json)| -> Result<_> {
                let payload: SessionArchivePayload = serde_json::from_str(&archive_json)
                    .with_context(|| format!("failed to deserialize save slot `{slot_id}` payload"))?;
                Ok(SaveSlotRecord {
                    slot_id,
                    session_id,
                    title,
                    created_at,
                    updated_at,
                    payload,
                })
            },
        )
        .transpose()
    }

    fn open_connection(&self) -> Result<Connection> {
        ensure_parent_dir(&self.db_path)?;
        Connection::open(&self.db_path)
            .with_context(|| format!("failed to open sqlite database at `{}`", self.db_path.display()))
    }

    fn existing_created_at(&self, conn: &Connection, slot_id: &str) -> Result<Option<String>> {
        conn.query_row(
            "SELECT created_at FROM save_slots WHERE slot_id = ?1",
            [slot_id],
            |row| row.get(0),
        )
        .optional()
        .with_context(|| format!("failed to query created_at for slot `{slot_id}`"))
    }
}

fn ensure_parent_dir(db_path: &Path) -> Result<()> {
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create sqlite parent directory `{}`",
                parent.display()
            )
        })?;
    }
    Ok(())
}

fn turn_phase_storage_value(payload: &SessionArchivePayload) -> Result<String> {
    let phase_value =
        serde_json::to_value(payload.turn_state.phase).context("failed to serialize turn phase")?;
    let Value::String(phase) = phase_value else {
        anyhow::bail!("serialized turn phase is not a string");
    };
    Ok(phase)
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    use agent::agent::context::Context;
    use akashic_ecs::resources::{
        history::SessionHistoryLog,
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    };
    use uuid::Uuid;

    use crate::api::archive::{ProtagonistDecisionArchive, TurnStateArchive};

    #[test]
    fn save_slot_round_trip_restores_archive_payload() {
        let db_path = std::env::temp_dir().join(format!("archive-db-{}.sqlite3", Uuid::new_v4()));
        let repo = ArchiveRepository::new(&db_path);
        repo.init_schema().expect("schema should initialize");

        let payload = sample_payload();
        repo.upsert_save_slot("slot-main", &payload)
            .expect("save slot should persist");

        let loaded = repo
            .load_archive_payload("slot-main")
            .expect("load should succeed")
            .expect("payload should exist");

        assert_eq!(loaded.session_id, payload.session_id);
        assert_eq!(loaded.title, payload.title);
        assert_eq!(loaded.turn_state.turn_index, payload.turn_state.turn_index);
        assert_eq!(
            loaded.protagonist_decision.committed_action,
            payload.protagonist_decision.committed_action
        );
        assert_eq!(
            loaded.world_snapshot.scene_title,
            payload.world_snapshot.scene_title
        );

        let _ = fs::remove_file(db_path);
    }

    fn sample_payload() -> SessionArchivePayload {
        SessionArchivePayload {
            session_id: "session-test".to_string(),
            title: "第3轮：雨夜档案馆".to_string(),
            world_profile: "world profile".to_string(),
            protagonist_profile: "protagonist profile".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayerChoice,
                turn_index: 3,
                active_turn_id: 3,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            world_snapshot: WorldSnapshot {
                round: 3,
                scene_title: "雨夜档案馆".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "躲进书架阴影".to_string(),
                choices: Vec::new(),
            },
            history_log: SessionHistoryLog::default(),
        }
    }
}
