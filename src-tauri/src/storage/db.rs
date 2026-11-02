// src-tauri/src/storage/db.rs
// SQLite connection pool and schema migrations.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

/// All SQL migrations in order. Each entry is applied exactly once,
/// identified by its 1-based index as the schema version.
const MIGRATIONS: &[&str] = &[
    // Migration 0001 — initial schema
    r#"
    CREATE TABLE IF NOT EXISTS local_blobs (
        hash             TEXT    PRIMARY KEY,
        size_bytes       INTEGER NOT NULL,
        last_accessed_at INTEGER NOT NULL,
        is_pinned        INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_last_accessed
        ON local_blobs(last_accessed_at);

    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    INSERT OR IGNORE INTO settings(key, value)
        VALUES ('cache_limit_bytes', '5368709120');

    INSERT OR IGNORE INTO settings(key, value)
        VALUES ('full_mirror', 'false');

    INSERT OR IGNORE INTO settings(key, value)
        VALUES ('relay_url', '');
    "#,
];

/// Thread-safe SQLite database wrapper.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the SQLite database at `path` and run any pending migrations.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening SQLite at {}", path.display()))?;

        // Enable WAL mode for better concurrent read performance.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Database {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create a simple user_version-based migration tracker.
        let current_version: i64 =
            conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

        let pending = &MIGRATIONS[current_version as usize..];
        for (i, sql) in pending.iter().enumerate() {
            tracing::info!("Running DB migration {}", current_version as usize + i + 1);
            conn.execute_batch(sql)?;
        }

        let new_version = MIGRATIONS.len() as i64;
        conn.execute_batch(&format!("PRAGMA user_version = {new_version}"))?;
        Ok(())
    }

    /// Retrieve a settings value by key. Returns `Ok(None)` if not set.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Upsert a settings value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Expose a reference to the inner connection for use by other modules.
    /// Callers must lock it themselves; this gives controlled access.
    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

