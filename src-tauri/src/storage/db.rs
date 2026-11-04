// src-tauri/src/storage/db.rs
// SQLite connection pool and schema migrations.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

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
    // Migration 0002 — add file name and owner node id for ownership and permission control
    r#"
    ALTER TABLE local_blobs ADD COLUMN name TEXT;
    ALTER TABLE local_blobs ADD COLUMN owner_node_id TEXT;
    "#,
    // Migration 0003 — community delete voting & moderation
    r#"
    CREATE TABLE IF NOT EXISTS file_delete_votes (
        hash          TEXT NOT NULL,
        voter_node_id TEXT NOT NULL,
        voted_at      INTEGER NOT NULL,
        PRIMARY KEY (hash, voter_node_id)
    );
    CREATE INDEX IF NOT EXISTS idx_delete_votes_hash ON file_delete_votes(hash);
    "#,
    // Migration 0004 — track active file seeds across the peer swarm
    r#"
    CREATE TABLE IF NOT EXISTS file_seeds (
        hash          TEXT NOT NULL,
        peer_node_id  TEXT NOT NULL,
        last_seen_at  INTEGER NOT NULL,
        PRIMARY KEY (hash, peer_node_id)
    );
    CREATE INDEX IF NOT EXISTS idx_file_seeds_hash ON file_seeds(hash);
    "#,
    // Migration 0005 — anti-delete keep voting and deletion grace period
    r#"
    CREATE TABLE IF NOT EXISTS file_keep_votes (
        hash          TEXT NOT NULL,
        voter_node_id TEXT NOT NULL,
        voted_at      INTEGER NOT NULL,
        PRIMARY KEY (hash, voter_node_id)
    );
    CREATE INDEX IF NOT EXISTS idx_keep_votes_hash ON file_keep_votes(hash);
    ALTER TABLE local_blobs ADD COLUMN deletion_scheduled_at INTEGER;
    "#,
    // Migration 0006 — network secret / workspace passphrase for swarm isolation
    r#"
    INSERT OR IGNORE INTO settings(key, value) VALUES ('network_secret', '');
    "#,
    // Migration 0007 — network SSID profile rules for campus stealth mode
    r#"
    CREATE TABLE IF NOT EXISTS network_ssid_rules (
        ssid     TEXT PRIMARY KEY,
        profile  TEXT NOT NULL CHECK(profile IN ('campus', 'trusted')),
        added_at INTEGER NOT NULL
    );
    INSERT OR IGNORE INTO network_ssid_rules(ssid, profile, added_at)
        VALUES ('eduroam', 'campus', unixepoch());
    INSERT OR IGNORE INTO network_ssid_rules(ssid, profile, added_at)
        VALUES ('campusroam', 'campus', unixepoch());
    INSERT OR IGNORE INTO settings(key, value) VALUES ('network_profile_override', 'auto');
    "#,
    // Migration 0008 — first-run onboarding status tracker
    r#"
    INSERT OR IGNORE INTO settings(key, value) VALUES ('onboarding_complete', 'false');
    "#,
    // Migration 0009 — SSD wear defense, transfer resume queue, and daily write budget
    r#"
    ALTER TABLE local_blobs ADD COLUMN written_at INTEGER;
    UPDATE local_blobs SET written_at = last_accessed_at WHERE written_at IS NULL;

    CREATE TABLE IF NOT EXISTS transfer_resume_queue (
        hash           TEXT PRIMARY KEY,
        name           TEXT,
        total_bytes    INTEGER NOT NULL,
        received_bytes INTEGER NOT NULL DEFAULT 0,
        source_peer    TEXT,
        created_at     INTEGER NOT NULL,
        updated_at     INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS write_budget (
        date_key      TEXT PRIMARY KEY,
        bytes_written INTEGER NOT NULL DEFAULT 0
    );
    "#,
    // Migration 0010 — study ratings, bookmarks, and emergency quarantine reports
    r#"
    CREATE TABLE IF NOT EXISTS file_ratings (
        hash          TEXT NOT NULL,
        voter_node_id TEXT NOT NULL,
        rating        INTEGER NOT NULL CHECK(rating IN (-1, 0, 1)),
        voted_at      INTEGER NOT NULL,
        PRIMARY KEY (hash, voter_node_id)
    );
    CREATE INDEX IF NOT EXISTS idx_ratings_hash ON file_ratings(hash);

    CREATE TABLE IF NOT EXISTS file_stars (
        hash          TEXT PRIMARY KEY,
        starred_at    INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS quarantine_reports (
        hash             TEXT NOT NULL,
        reporter_node_id TEXT NOT NULL,
        reported_at      INTEGER NOT NULL,
        PRIMARY KEY (hash, reporter_node_id)
    );
    CREATE INDEX IF NOT EXISTS idx_quarantine_hash ON quarantine_reports(hash);

    ALTER TABLE local_blobs ADD COLUMN is_quarantined   INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE local_blobs ADD COLUMN quarantine_expires_at INTEGER;
    "#,
    // Migration 0011 — bilateral cryptographic seeding receipts
    r#"
    CREATE TABLE IF NOT EXISTS seeding_receipts (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        hash              TEXT NOT NULL,
        seeder_node_id    TEXT NOT NULL,
        recipient_node_id TEXT NOT NULL,
        bytes_transferred INTEGER NOT NULL,
        issued_at         INTEGER NOT NULL,
        signature         TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_receipts_seeder ON seeding_receipts(seeder_node_id, issued_at);
    CREATE INDEX IF NOT EXISTS idx_receipts_hash   ON seeding_receipts(hash);
    "#,
    // Migration 0012 — catalog lifecycle aging metadata
    r#"
    ALTER TABLE local_blobs ADD COLUMN catalog_first_seen_at INTEGER;
    ALTER TABLE local_blobs ADD COLUMN last_any_seed_seen_at INTEGER;
    UPDATE local_blobs SET catalog_first_seen_at = last_accessed_at WHERE catalog_first_seen_at IS NULL;
    "#,
];

/// Thread-safe SQLite database wrapper.
#[derive(Debug)]
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
        let current_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

        let current_ver = current_version.max(0) as usize;
        if current_ver < MIGRATIONS.len() {
            let pending = &MIGRATIONS[current_ver..];
            for (i, sql) in pending.iter().enumerate() {
                tracing::info!("Running DB migration {}", current_ver + i + 1);
                conn.execute_batch(sql)?;
            }

            let new_version = MIGRATIONS.len() as i64;
            conn.execute_batch(&format!("PRAGMA user_version = {new_version}"))?;
        }
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
