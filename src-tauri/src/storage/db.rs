// src-tauri/src/storage/db.rs
// SQLite connection pool, schema migrations, and per-network database management.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};

/// Information about a network database instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDatabaseInfo {
    pub network_secret: String,
    pub display_name: String,
    pub db_file: String,
    pub file_count: usize,
    pub seedable_file_count: usize,
    pub is_active: bool,
}

/// Summary of a cross-network file transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSummary {
    pub transferred_count: usize,
    pub skipped_cloud_only_count: usize,
    pub total_bytes_transferred: u64,
}

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
    // Migration 0013 — persistent virtual folders (including empty folders)
    r#"
    CREATE TABLE IF NOT EXISTS virtual_folders (
        path       TEXT PRIMARY KEY,
        created_at INTEGER NOT NULL
    );
    "#,
    // Migration 0014 — Swarm peer version tracking for median swarm version calculation
    r#"
    CREATE TABLE IF NOT EXISTS peer_versions (
        node_id      TEXT PRIMARY KEY,
        app_version  TEXT NOT NULL,
        last_seen_at INTEGER NOT NULL
    );
    "#,
    // Migration 0015 — Enforce Western Europe (EU) relay default
    r#"
    UPDATE settings SET value = 'https://euw1-1.relay.iroh.network/'
    WHERE key = 'relay_url' AND (value LIKE '%use1-1%' OR value = '' OR value = 'default' OR value = 'global');
    "#,
    // Migration 0016 — Migrate obsolete Iroh 0.x relay URLs to Iroh 1.4 infrastructure
    r#"
    UPDATE settings SET value = 'https://euc1-1.relay.n0.iroh.link/'
    WHERE key = 'relay_url' AND (value LIKE '%iroh.network%' OR value LIKE '%euw1-1%' OR value = '' OR value = 'default' OR value = 'global');
    "#,
    // Migration 0017 — Cryptographically signed deletion tombstones for authoritative owner deletions
    r#"
    CREATE TABLE IF NOT EXISTS deleted_blobs (
        hash          TEXT PRIMARY KEY,
        owner_node_id TEXT NOT NULL,
        deleted_at    INTEGER NOT NULL,
        signature     TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_deleted_blobs_owner ON deleted_blobs(owner_node_id);
    "#,
];

fn apply_migrations_to_conn(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA wal_autocheckpoint=1000;")?;
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

/// Compare two node IDs for equivalence regardless of whether they are formatted
/// as base32 or canonical hex.
pub fn is_matching_node_id(id1: &str, id2: &str) -> bool {
    let t1 = id1.trim();
    let t2 = id2.trim();
    if t1.eq_ignore_ascii_case(t2) {
        return true;
    }
    match (t1.parse::<iroh::EndpointId>(), t2.parse::<iroh::EndpointId>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Convert any base32 or hex NodeId string into its canonical hex representation.
pub fn to_canonical_node_id(input: &str) -> String {
    let trimmed = input.trim();
    if let Ok(id) = trimmed.parse::<iroh::EndpointId>() {
        id.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Migrate and normalize all stored node IDs across SQLite tables to canonical hex format.
pub fn canonicalize_node_ids_in_conn(conn: &Connection) -> Result<()> {
    // 1. local_blobs.owner_node_id
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT owner_node_id FROM local_blobs WHERE owner_node_id IS NOT NULL AND owner_node_id != ''") {
        if let Ok(owners) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            let list: Vec<String> = owners.filter_map(|r| r.ok()).collect();
            for owner in list {
                if let Ok(id) = owner.parse::<iroh::EndpointId>() {
                    let canon = id.to_string();
                    if canon != owner {
                        let _ = conn.execute(
                            "UPDATE local_blobs SET owner_node_id = ?1 WHERE owner_node_id = ?2",
                            rusqlite::params![canon, owner],
                        );
                    }
                }
            }
        }
    }

    // 2. file_seeds.peer_node_id
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT peer_node_id FROM file_seeds WHERE peer_node_id IS NOT NULL AND peer_node_id != ''") {
        if let Ok(seeders) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            let list: Vec<String> = seeders.filter_map(|r| r.ok()).collect();
            for seeder in list {
                if let Ok(id) = seeder.parse::<iroh::EndpointId>() {
                    let canon = id.to_string();
                    if canon != seeder {
                        let _ = conn.execute(
                            "UPDATE OR IGNORE file_seeds SET peer_node_id = ?1 WHERE peer_node_id = ?2",
                            rusqlite::params![canon, seeder],
                        );
                        let _ = conn.execute(
                            "DELETE FROM file_seeds WHERE peer_node_id = ?1",
                            rusqlite::params![seeder],
                        );
                    }
                }
            }
        }
    }

    // 3. file_delete_votes, file_keep_votes, file_ratings
    for table in &["file_delete_votes", "file_keep_votes", "file_ratings"] {
        let col = "voter_node_id";
        let query = format!("SELECT DISTINCT {col} FROM {table} WHERE {col} IS NOT NULL AND {col} != ''");
        if let Ok(mut stmt) = conn.prepare(&query) {
            if let Ok(voters) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                let list: Vec<String> = voters.filter_map(|r| r.ok()).collect();
                for voter in list {
                    if let Ok(id) = voter.parse::<iroh::EndpointId>() {
                        let canon = id.to_string();
                        if canon != voter {
                            let _ = conn.execute(
                                &format!("UPDATE OR IGNORE {table} SET {col} = ?1 WHERE {col} = ?2"),
                                rusqlite::params![canon, voter],
                            );
                            let _ = conn.execute(
                                &format!("DELETE FROM {table} WHERE {col} = ?1"),
                                rusqlite::params![voter],
                            );
                        }
                    }
                }
            }
        }
    }

    // 4. quarantine_reports
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT reporter_node_id FROM quarantine_reports WHERE reporter_node_id IS NOT NULL AND reporter_node_id != ''") {
        if let Ok(reporters) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            let list: Vec<String> = reporters.filter_map(|r| r.ok()).collect();
            for rep in list {
                if let Ok(id) = rep.parse::<iroh::EndpointId>() {
                    let canon = id.to_string();
                    if canon != rep {
                        let _ = conn.execute(
                            "UPDATE OR IGNORE quarantine_reports SET reporter_node_id = ?1 WHERE reporter_node_id = ?2",
                            rusqlite::params![canon, rep],
                        );
                        let _ = conn.execute(
                            "DELETE FROM quarantine_reports WHERE reporter_node_id = ?1",
                            rusqlite::params![rep],
                        );
                    }
                }
            }
        }
    }

    // 5. seeding_receipts
    for col in &["seeder_node_id", "recipient_node_id"] {
        let query = format!("SELECT DISTINCT {col} FROM seeding_receipts WHERE {col} IS NOT NULL AND {col} != ''");
        if let Ok(mut stmt) = conn.prepare(&query) {
            if let Ok(nodes) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                let list: Vec<String> = nodes.filter_map(|r| r.ok()).collect();
                for node in list {
                    if let Ok(id) = node.parse::<iroh::EndpointId>() {
                        let canon = id.to_string();
                        if canon != node {
                            let _ = conn.execute(
                                &format!("UPDATE seeding_receipts SET {col} = ?1 WHERE {col} = ?2"),
                                rusqlite::params![canon, node],
                            );
                        }
                    }
                }
            }
        }
    }

    // 6. peer_versions
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT node_id FROM peer_versions WHERE node_id IS NOT NULL AND node_id != ''") {
        if let Ok(nodes) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            let list: Vec<String> = nodes.filter_map(|r| r.ok()).collect();
            for node in list {
                if let Ok(id) = node.parse::<iroh::EndpointId>() {
                    let canon = id.to_string();
                    if canon != node {
                        let _ = conn.execute(
                            "UPDATE OR IGNORE peer_versions SET node_id = ?1 WHERE node_id = ?2",
                            rusqlite::params![canon, node],
                        );
                        let _ = conn.execute(
                            "DELETE FROM peer_versions WHERE node_id = ?1",
                            rusqlite::params![node],
                        );
                    }
                }
            }
        }
    }

    // 7. bootstrap_peer in settings
    if let Ok(val) = conn.query_row::<String, _, _>(
        "SELECT value FROM settings WHERE key = 'bootstrap_peer'",
        [],
        |r| r.get(0),
    ) {
        if let Ok(id) = val.trim().parse::<iroh::EndpointId>() {
            let canon = id.to_string();
            if canon != val.trim() {
                let _ = conn.execute(
                    "UPDATE settings SET value = ?1 WHERE key = 'bootstrap_peer'",
                    rusqlite::params![canon],
                );
            }
        }
    }

    Ok(())
}

/// Thread-safe SQLite database wrapper supporting multi-network isolation.
#[derive(Debug)]
pub struct Database {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
    active_network_secret: Mutex<String>,
}

impl Database {
    /// Open the SQLite database for the active or default network and run pending migrations.
    pub fn open(path: &Path) -> Result<Self> {
        let data_dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };

        // Determine active network secret from active_network.txt if present
        let active_txt = data_dir.join("active_network.txt");
        let initial_secret = if active_txt.exists() {
            fs::read_to_string(&active_txt).unwrap_or_default().trim().to_string()
        } else {
            String::new()
        };

        let target_db_file = if initial_secret.is_empty() {
            if data_dir.join("p2pstud.db").exists() {
                data_dir.join("p2pstud.db")
            } else {
                data_dir.join("p2pstud_default.db")
            }
        } else {
            let hash = iroh_blobs::Hash::new(format!("p2pstud-net-secret:{initial_secret}").as_bytes());
            let short = &hash.to_hex()[..16];
            data_dir.join(format!("p2pstud_net_{short}.db"))
        };

        let conn = Connection::open(&target_db_file)
            .with_context(|| format!("opening SQLite at {}", target_db_file.display()))?;

        apply_migrations_to_conn(&conn)?;
        canonicalize_node_ids_in_conn(&conn)?;

        let db = Database {
            conn: Mutex::new(conn),
            data_dir,
            active_network_secret: Mutex::new(initial_secret.clone()),
        };

        let _ = db.register_known_network(&initial_secret);
        Ok(db)
    }

    /// Return the currently active network secret passphrase.
    pub fn active_network_secret(&self) -> String {
        self.active_network_secret.lock().unwrap().clone()
    }

    /// Resolve the database path for a given network secret.
    pub fn get_network_db_path(&self, secret: &str) -> PathBuf {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            if self.data_dir.join("p2pstud_default.db").exists() {
                self.data_dir.join("p2pstud_default.db")
            } else {
                self.data_dir.join("p2pstud.db")
            }
        } else {
            let hash = iroh_blobs::Hash::new(format!("p2pstud-net-secret:{trimmed}").as_bytes());
            let short = &hash.to_hex()[..16];
            self.data_dir.join(format!("p2pstud_net_{short}.db"))
        }
    }

    /// Register a network secret in known_networks.json.
    fn register_known_network(&self, secret: &str) -> Result<()> {
        let registry_path = self.data_dir.join("known_networks.json");
        let mut known: Vec<String> = if registry_path.exists() {
            let content = fs::read_to_string(&registry_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        let trimmed = secret.trim().to_string();
        if !known.contains(&trimmed) {
            known.push(trimmed);
            let json = serde_json::to_string_pretty(&known)?;
            let _ = fs::write(&registry_path, json);
        }
        Ok(())
    }

    /// Switch active network database.
    pub fn switch_network(&self, new_secret: &str) -> Result<()> {
        let trimmed = new_secret.trim().to_string();
        let target_db_file = self.get_network_db_path(&trimmed);

        let new_conn = Connection::open(&target_db_file)
            .with_context(|| format!("opening target database {}", target_db_file.display()))?;

        apply_migrations_to_conn(&new_conn)?;
        canonicalize_node_ids_in_conn(&new_conn)?;

        // Ensure settings in target DB record its own network_secret
        new_conn.execute(
            "INSERT INTO settings(key, value) VALUES('network_secret', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![&trimmed],
        )?;

        // Atomically replace active connection
        {
            let mut conn_guard = self.conn.lock().unwrap();
            *conn_guard = new_conn;
        }

        {
            let mut secret_guard = self.active_network_secret.lock().unwrap();
            *secret_guard = trimmed.clone();
        }

        // Persist active network selection
        let active_txt = self.data_dir.join("active_network.txt");
        let _ = fs::write(&active_txt, &trimmed);
        let _ = self.register_known_network(&trimmed);

        Ok(())
    }

    /// List all known and on-disk network databases.
    pub fn list_network_databases(&self) -> Result<Vec<NetworkDatabaseInfo>> {
        let registry_path = self.data_dir.join("known_networks.json");
        let mut known: HashSet<String> = if registry_path.exists() {
            let content = fs::read_to_string(&registry_path).unwrap_or_default();
            serde_json::from_str::<Vec<String>>(&content)
                .unwrap_or_default()
                .into_iter()
                .collect()
        } else {
            HashSet::new()
        };

        // Always include default empty network and active network
        known.insert(String::new());
        known.insert(self.active_network_secret());

        let active_secret = self.active_network_secret();
        let mut results = Vec::new();

        for secret in known {
            let db_path = self.get_network_db_path(&secret);
            let db_file = db_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "p2pstud.db".to_string());

            let is_active = secret == active_secret;
            let display_name = if secret.is_empty() {
                "Default Network (Open Swarm)".to_string()
            } else {
                secret.clone()
            };

            let (file_count, seedable_file_count) = if is_active {
                let conn = self.conn.lock().unwrap();
                let fc: i64 = conn
                    .query_row("SELECT COUNT(*) FROM local_blobs", [], |r| r.get(0))
                    .unwrap_or(0);
                let sfc: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM local_blobs WHERE is_pinned = 1 OR (written_at IS NOT NULL AND written_at > 0)",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                (fc.max(0) as usize, sfc.max(0) as usize)
            } else if db_path.exists() {
                if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                    let fc: i64 = conn
                        .query_row("SELECT COUNT(*) FROM local_blobs", [], |r| r.get(0))
                        .unwrap_or(0);
                    let sfc: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM local_blobs WHERE is_pinned = 1 OR (written_at IS NOT NULL AND written_at > 0)",
                            [],
                            |r| r.get(0),
                        )
                        .unwrap_or(0);
                    (fc.max(0) as usize, sfc.max(0) as usize)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };

            results.push(NetworkDatabaseInfo {
                network_secret: secret,
                display_name,
                db_file,
                file_count,
                seedable_file_count,
                is_active,
            });
        }

        results.sort_by(|a, b| {
            if a.is_active {
                std::cmp::Ordering::Less
            } else if b.is_active {
                std::cmp::Ordering::Greater
            } else {
                a.display_name.cmp(&b.display_name)
            }
        });

        Ok(results)
    }

    /// Transfer files from source network to target network.
    /// STRICT RULE (per issue requirements):
    /// "only transfer files you can actually seed (and you should become owner) otherwise you make dead files on the server"
    pub fn transfer_files_to_network(
        &self,
        source_secret: &str,
        target_secret: &str,
        our_node_id: &str,
    ) -> Result<TransferSummary> {
        let src_trimmed = source_secret.trim();
        let tgt_trimmed = target_secret.trim();

        if src_trimmed == tgt_trimmed {
            anyhow::bail!("Source network and target network cannot be identical.");
        }

        let src_path = self.get_network_db_path(src_trimmed);
        let tgt_path = self.get_network_db_path(tgt_trimmed);

        if !src_path.exists() {
            anyhow::bail!("Source database does not exist: {}", src_path.display());
        }

        // Open target connection if not active
        let target_conn = Connection::open(&tgt_path)?;
        apply_migrations_to_conn(&target_conn)?;
        canonicalize_node_ids_in_conn(&target_conn)?;

        // Read candidate files from source
        let source_conn = Connection::open_with_flags(&src_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        struct SourceFile {
            hash: String,
            name: Option<String>,
            size_bytes: i64,
            is_pinned: i64,
            is_seedable: bool,
        }

        let mut stmt = source_conn.prepare(
            "SELECT hash, name, size_bytes, is_pinned, written_at FROM local_blobs",
        )?;

        let rows = stmt.query_map([], |row| {
            let is_pinned: i64 = row.get(3)?;
            let written_at: Option<i64> = row.get(4)?;
            // Only files cached locally or pinned are seedable
            let is_seedable = is_pinned == 1 || written_at.is_some();
            Ok(SourceFile {
                hash: row.get(0)?,
                name: row.get(1)?,
                size_bytes: row.get(2)?,
                is_pinned,
                is_seedable,
            })
        })?;

        let mut transferred_count = 0usize;
        let mut skipped_cloud_only_count = 0usize;
        let mut total_bytes_transferred = 0u64;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        for item_res in rows {
            let file = item_res?;
            if !file.is_seedable {
                // Rule: Skip files you cannot seed to prevent dead files on the new swarm
                skipped_cloud_only_count += 1;
                continue;
            }

            // Insert into target local_blobs: this node becomes the owner and seeder!
            target_conn.execute(
                "INSERT INTO local_blobs (hash, size_bytes, last_accessed_at, is_pinned, name, owner_node_id, written_at, catalog_first_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(hash) DO UPDATE SET
                    name = excluded.name,
                    owner_node_id = excluded.owner_node_id,
                    last_accessed_at = excluded.last_accessed_at,
                    is_pinned = excluded.is_pinned",
                params![
                    &file.hash,
                    file.size_bytes,
                    now,
                    file.is_pinned,
                    &file.name,
                    our_node_id,
                    now,
                    now,
                ],
            )?;

            // Register our node as an active seed in the target network
            target_conn.execute(
                "INSERT INTO file_seeds (hash, peer_node_id, last_seen_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(hash, peer_node_id) DO UPDATE SET last_seen_at = excluded.last_seen_at",
                params![&file.hash, our_node_id, now],
            )?;

            transferred_count += 1;
            total_bytes_transferred += file.size_bytes.max(0) as u64;
        }

        // Also copy virtual folder structure from source to target
        if let Ok(mut folder_stmt) = source_conn.prepare("SELECT path FROM virtual_folders") {
            if let Ok(folder_rows) = folder_stmt.query_map([], |r| r.get::<_, String>(0)) {
                for folder in folder_rows.flatten() {
                    let _ = target_conn.execute(
                        "INSERT OR IGNORE INTO virtual_folders (path, created_at) VALUES (?1, ?2)",
                        params![&folder, now],
                    );
                }
            }
        }

        // If target is the currently active database, reload its connection to reflect new records
        if tgt_trimmed == self.active_network_secret() {
            let _ = self.switch_network(tgt_trimmed);
        }

        Ok(TransferSummary {
            transferred_count,
            skipped_cloud_only_count,
            total_bytes_transferred,
        })
    }

    /// Delete a non-active network database.
    pub fn delete_network_database(&self, secret: &str) -> Result<()> {
        let trimmed = secret.trim();
        if trimmed == self.active_network_secret() {
            anyhow::bail!("Cannot delete the active network database. Switch to another network first.");
        }

        let db_path = self.get_network_db_path(trimmed);
        if db_path.exists() {
            let _ = fs::remove_file(&db_path);
            let _ = fs::remove_file(format!("{}-wal", db_path.display()));
            let _ = fs::remove_file(format!("{}-shm", db_path.display()));
        }

        // Update known_networks.json
        let registry_path = self.data_dir.join("known_networks.json");
        if registry_path.exists() {
            let content = fs::read_to_string(&registry_path).unwrap_or_default();
            let mut known: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
            known.retain(|s| s != trimmed);
            let json = serde_json::to_string_pretty(&known)?;
            let _ = fs::write(&registry_path, json);
        }

        Ok(())
    }

    /// Clear all file catalog records, virtual folders, and seeds from the active network database.
    pub fn clear_active_database(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "DELETE FROM file_seeds;
             DELETE FROM files;
             DELETE FROM local_blobs;
             DELETE FROM virtual_folders;
             DELETE FROM file_keep_votes;
             DELETE FROM file_ratings;
             DELETE FROM file_stars;
             DELETE FROM quarantine_reports;
             DELETE FROM transfer_resume_queue;
             DELETE FROM deleted_blobs;",
        )?;
        drop(conn);

        // Also purge physical blob files from disk to match the cleared database
        let blob_dir = self.resolve_blob_dir();
        let _ = crate::storage::path_utils::cleanup_orphaned_disk_blobs(&blob_dir, self);

        Ok(())
    }

    /// Return the base data directory for this database.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Resolve the active blob storage directory (respecting custom_blob_dir setting).
    pub fn resolve_blob_dir(&self) -> PathBuf {
        match self.get_setting("custom_blob_dir") {
            Ok(Some(custom)) if !custom.trim().is_empty() => PathBuf::from(custom.trim()),
            _ => self.data_dir.join("blobs"),
        }
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

    /// Delete a settings value by key.
    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    /// Checkpoints and truncates the SQLite WAL file to reclaim disk space and prevent WAL bloat.
    pub fn checkpoint_wal(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    /// Checkpoints WAL and vacuums the active database to reclaim fragmented space.
    pub fn vacuum_db(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(())
    }

    /// Return (db_file_bytes, wal_file_bytes) for the currently active database.
    pub fn get_database_sizes(&self) -> (u64, u64) {
        let secret = self.active_network_secret();
        let db_path = self.get_network_db_path(&secret);
        let wal_path = PathBuf::from(format!("{}-wal", db_path.display()));

        let db_bytes = fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
        let wal_bytes = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        (db_bytes, wal_bytes)
    }

    /// Expose a reference to the inner connection for use by other modules.
    /// Callers must lock it themselves; this gives controlled access.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_network_database_isolation_and_transfer() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("p2pstud.db");
        let db = Database::open(&db_path).unwrap();

        assert_eq!(db.active_network_secret(), "");

        // Add 2 files to default network: one pinned (seedable), one cloud_only (not seedable)
        {
            let conn = db.lock();
            conn.execute(
                "INSERT INTO local_blobs (hash, size_bytes, last_accessed_at, is_pinned, name, owner_node_id, written_at)
                 VALUES ('hash_seedable_1', 1000, 100, 1, 'PinnedFile.pdf', 'node_1', 100)",
                [],
            ).unwrap();
            conn.execute(
                "INSERT INTO local_blobs (hash, size_bytes, last_accessed_at, is_pinned, name, owner_node_id, written_at)
                 VALUES ('hash_cloud_only_2', 2000, 100, 0, 'CloudOnly.pdf', 'node_remote', NULL)",
                [],
            ).unwrap();
        }

        // Check file counts in default network
        {
            let conn = db.lock();
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM local_blobs", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 2);
        }

        // Switch to a new network: "course-cs-2026"
        db.switch_network("course-cs-2026").unwrap();
        assert_eq!(db.active_network_secret(), "course-cs-2026");

        // The new network database MUST be completely isolated (0 files)
        {
            let conn = db.lock();
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM local_blobs", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 0, "New network database must be empty and isolated");
        }

        // List databases: should see default and course-cs-2026
        let dbs = db.list_network_databases().unwrap();
        assert!(dbs.len() >= 2);
        let default_db = dbs.iter().find(|d| d.network_secret.is_empty()).unwrap();
        assert_eq!(default_db.file_count, 2);
        assert_eq!(default_db.seedable_file_count, 1);

        let cs_db = dbs.iter().find(|d| d.network_secret == "course-cs-2026").unwrap();
        assert!(cs_db.is_active);
        assert_eq!(cs_db.file_count, 0);

        // Transfer files from default to course-cs-2026
        // Rule: Only transfer files you can actually seed (pinned or written_at set), and become owner!
        let summary = db.transfer_files_to_network("", "course-cs-2026", "my_node_id").unwrap();
        assert_eq!(summary.transferred_count, 1, "Only 1 file was seedable");
        assert_eq!(summary.skipped_cloud_only_count, 1, "Cloud-only file must be skipped");
        assert_eq!(summary.total_bytes_transferred, 1000);

        // Verify the transferred file in course-cs-2026
        {
            let conn = db.lock();
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM local_blobs", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 1);

            let (owner, seed_count): (String, i64) = conn.query_row(
                "SELECT b.owner_node_id, (SELECT COUNT(*) FROM file_seeds s WHERE s.hash = b.hash AND s.peer_node_id = 'my_node_id')
                 FROM local_blobs b WHERE b.hash = 'hash_seedable_1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap();
            assert_eq!(owner, "my_node_id", "Transferred file must set our node as owner");
            assert_eq!(seed_count, 1, "Our node must be registered as seeder in target network");
        }

        // Switch back to default
        db.switch_network("").unwrap();
        assert_eq!(db.active_network_secret(), "");

        // Delete course-cs-2026 database
        db.delete_network_database("course-cs-2026").unwrap();
        let dbs_after = db.list_network_databases().unwrap();
        assert!(dbs_after.iter().all(|d| d.network_secret != "course-cs-2026"));
    }
}
