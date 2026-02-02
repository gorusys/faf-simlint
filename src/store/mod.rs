//! SQLite persistence for scan history and diffs.

use crate::model::UnitSummary;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS scans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_dir TEXT NOT NULL,
    created_at TEXT NOT NULL,
    summary_json TEXT
);

CREATE TABLE IF NOT EXISTS scan_units (
    scan_id INTEGER NOT NULL REFERENCES scans(id),
    unit_id TEXT NOT NULL,
    summary_json TEXT NOT NULL,
    PRIMARY KEY (scan_id, unit_id)
);

CREATE INDEX IF NOT EXISTS idx_scan_units_scan ON scan_units(scan_id);
";

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Store { conn })
    }

    pub fn insert_scan(&self, data_dir: &str, units: &[UnitSummary]) -> Result<i64, String> {
        let now: DateTime<Utc> = Utc::now();
        let created = now.to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO scans (data_dir, created_at) VALUES (?1, ?2)",
                params![data_dir, created],
            )
            .map_err(|e| e.to_string())?;
        let id = self.conn.last_insert_rowid();

        let mut stmt = self
            .conn
            .prepare("INSERT INTO scan_units (scan_id, unit_id, summary_json) VALUES (?1, ?2, ?3)")
            .map_err(|e| e.to_string())?;
        for u in units {
            let json = serde_json::to_string(u).map_err(|e| e.to_string())?;
            stmt.execute(params![id, u.unit_id.id, json])
                .map_err(|e| e.to_string())?;
        }
        Ok(id)
    }

    pub fn list_scans(&self) -> Result<Vec<(i64, String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, data_dir, created_at FROM scans ORDER BY id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_scan_units(&self, scan_id: i64) -> Result<Vec<UnitSummary>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT summary_json FROM scan_units WHERE scan_id = ?1 ORDER BY unit_id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![scan_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let json: String = row.map_err(|e| e.to_string())?;
            let u: UnitSummary = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            out.push(u);
        }
        Ok(out)
    }
}
