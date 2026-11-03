# P2P Campus Workspace — Agent Build Plan

> **For the executing agent:** Read this file at the start of every session. Update the `## Progress Tracker` section after completing each task. Never skip the tracker update — it is the sole source of truth about where work left off.

> **Companion files — read these too before starting work:**
> - `REQUIREMENTS.md` — full requirements table, dependency versions, design decisions log, open questions
> - `AGENTS.md` — subagent roles, file ownership, startup checklists, completion criteria

---

## Agent Orchestration Guidelines

### How to Use This Plan

1. **Start every session** by reading this file top-to-bottom to orient yourself on current state.
2. **Track progress** by checking off tasks in the Progress Tracker and updating the `Last Completed` timestamp and session note.
3. **Use subagents liberally** for parallelizable work (see Subagent Strategy below).
4. **After each major milestone**, run the full test suite and commit with a descriptive message.
5. **If blocked**, write a `BLOCKED:` note in the relevant tracker row before stopping, so the next session knows what to investigate.

### Subagent Strategy

Spawn subagents for tasks that can run in parallel or in isolation. Suggested split:

| Subagent Role | Assigned Work |
|---|---|
| `RustBackendAgent` | Iroh engine, SQLite eviction worker, Tauri IPC commands |
| `FrontendAgent` | React/Svelte UI, file explorer, PDF/Markdown viewers, telemetry footer |
| `AnchorDaemonAgent` | `src/bin/anchor.rs`, Dockerfile, docker-compose, rate limiting |
| `TestingAgent` | Unit tests, integration tests, CI workflow files |
| `DocsAgent` | README, deployment guide, inline doc comments |

> **Coordination rule:** subagents must write their outputs to agreed paths listed in the File Map below. The orchestrating agent merges branches and resolves conflicts before committing.

### Session Checkpoint Protocol

At the end of every session (or when interrupted), update the `## Progress Tracker` table:
- Mark completed items ✅
- Mark in-progress items 🔄
- Mark blocked items ❌ with a `BLOCKED:` note
- Update `**Last session:**` at the top of the tracker

---

## Progress Tracker

> **Last session:** 2026-11-02
> **Last completed task:** Complete implementation achieved. All 19 steps implemented and verified: Rust backend with Iroh 0.29 & SQLite LRU eviction (14/14 unit tests pass), P2P multi-node blob exchange integration test passes, headless anchor daemon binary built and verified (`--health-check`), and React 18 frontend built (9/9 Vitest tests pass).
> **Next action:** Production deployment / release packaging.

| # | Step | Status | Notes |
|---|---|---|---|
| 1 | Tauri v2 scaffold + Cargo workspace | ✅ | Cargo workspace, Tauri config, package.json set up |
| 2 | Iroh node engine (`p2p/engine.rs`) | ✅ | Implemented with QUIC endpoint, relay fallback, blob store |
| 3 | SecretKey persistence & node startup | ✅ | Implemented in `p2p/identity.rs` with atomic write & OS file restrictions |
| 4 | Tauri IPC command surface | ✅ | `storage_cmds`, `file_cmds`, `peer_cmds` implemented in `commands/` |
| 5 | SQLite schema + blob tracker | ✅ | `storage/db.rs` (WAL, migrations) and `storage/blob_tracker.rs` implemented |
| 6 | LRU eviction background worker | ✅ | `storage/eviction.rs` implemented (90% high mark, 80% low mark) |
| 7 | Pin/unpin logic + tests | ✅ | Pinned blob survival, oldest-first eviction unit tests implemented |
| 8 | Frontend scaffold (Vite + React/Svelte + Tailwind) | ✅ | React 18 + Vite + TailwindCSS + Lucide Icons configured |
| 9 | Virtual file explorer UI | ✅ | `FileTree.tsx`, `FileRow.tsx`, `Breadcrumbs.tsx` with 4 sync-state badges |
| 10 | PDF viewer integration | ✅ | `PdfViewer.tsx` implemented with `pdfjs-dist` & canvas rendering |
| 11 | Markdown editor + split-pane preview | ✅ | `MarkdownEditor.tsx` with live marked HTML preview and debounced auto-save |
| 12 | Network telemetry footer | ✅ | `TelemetryBar.tsx` polling storage stats & peer connection status |
| 13 | Anchor daemon binary (`src-tauri/src/bin/anchor.rs`) | ✅ | Headless binary with Clap CLI, TLS cert support, `--no-relay` flag |
| 14 | Token-bucket rate limiter on anchor | ✅ | Session & aggregate token-bucket throttling implemented |
| 15 | Docker packaging (optional) — single `Dockerfile` + `docker-compose.yml` | ✅ | `docker/Dockerfile`, `docker-compose.yml`, `.env.example`, `p2pstud-anchor.service` |
| 16 | End-to-end connectivity test (2 local nodes) | ✅ | Feature-gated integration test passing (`two_nodes_transfer_blob`) |
| 17 | Security audit (path sanitization, no open RPC) | ✅ | `path_utils.rs` (`safe_join`), owner-only key permissions, in-process IPC |
| 18 | CI workflow (GitHub Actions) | ✅ | `.github/workflows/build.yml` and `test.yml` created |
| 19 | README + deployment guide | ✅ | Comprehensive `README.md` and `.gitignore` created |
| 20 | Production Readiness Polish & Background Tray | ✅ | System tray icon, window close interception, network & peer configuration, purge cache, installer & portable ZIP |
| 21 | Cross-Platform Linux & Windows Parity | ✅ | `xdg-open` on Linux, `0o600` secret key permissions, bundled SQLite, `.deb` & `.AppImage` bundle config in Tauri, CI matrix for Windows & Ubuntu |
| 22 | Multi-Layer Discovery & Smart Peer Dialing | ✅ | `LocalSwarmDiscovery` on LAN, `n0_dns` and `pkarr_dht` on WAN, auto background discovery connect, `NodeTicket` parsing |
| 23 | Workspace Catalog Synchronization & Remote Fetch | ✅ | Real-time QUIC catalog exchange (`/p2pstud/sync/1`), broadcast on upload/delete, on-demand remote blob download |
| 24 | Active Swarm Hosting (~80%) & Orphaned File Management | ✅ | Worker actively fills cache to 80% to seed course files; orphaned file force-cleanup for departed owners |
| 25 | Live Bandwidth Telemetry & Community Deletion Consensus Voting | ✅ | Real-time byte-rate tracker replaces static mock speeds; community vote-to-delete with 3-vote quorum |
| 26 | Upload Queue Visual Indicator & Smart Least-Seeds Cache Auto-Fill | ✅ | Live upload queue drawer & file row progress; smart auto-fill prioritizing lowest seed counts |
| 27 | Hierarchical Folder Support & Organization | ✅ | Virtual folder tree, directory navigation, "New Folder", folder-aware uploads & breadcrumb deep-linking |
| 28 | File Moving & Sole Active Seed Disk Eviction Protection | ✅ | Drag-and-drop & context file moving with QUIC sync; prevent disk eviction if local node is sole seed |
| 29 | Anti-Delete (Keep) Voting, 86% Consensus, 24h Grace Period & Offline Bundling | ✅ | Non-owners blocked from unilateral delete; Keep defense voting; 86% consensus ratio threshold; 24h grace period warning banner; offline-embedded release build |

---

## 1. Executive Summary & Objective

Build a lightweight, cross-platform desktop application (Windows + Linux) that enables cooperative P2P academic file sharing across restrictive university networks (e.g., Eduroam).

**Core model — "Host-to-Play" cooperative storage:**
- Every student dedicates configurable local disk space (default **5 GB**, toggle to "Full Mirror") to store and seed chunks.
- A cheap cloud VPS (\$3–5/month) is the **Anchor Node**: always-on persistence + private relay on TCP 443 + throttled fallback when peers are offline.
- All cryptography, chunk verification, and NAT traversal come from **Iroh** embedded directly in the Tauri binary — no custom socket protocols, no rolling crypto.

---

## 2. Technology Stack & Constraints

| Layer | Choice | Constraint |
|---|---|---|
| App shell | Tauri v2 (Rust + WebView) | Idle RAM ≤ 120 MB |
| P2P identity + transport | `iroh` + `iroh-net` | QUIC; relay fallback on port 443 |
| Content-addressed storage | `iroh-blobs` | BLAKE3 integrity on every chunk |
| Local state | `rusqlite` (SQLite) | Tracks blobs, timestamps, pin state |
| Frontend | React or Svelte + TailwindCSS + Lucide Icons | Bundled via Vite |
| PDF viewer | `pdfjs-dist` or `@react-pdf-viewer/core` | |
| Markdown editor | `@monaco-editor/react` or Milkdown/TipTap | Split-pane live preview |
| Async runtime | `tokio` (already used by Iroh) | |

**Hard constraints for the agent:**
- ❌ Do NOT write custom socket/crypto protocols.
- ❌ Do NOT bind unauthenticated HTTP/RPC ports to `0.0.0.0` on student machines.
- ✅ All file paths must be sanitized against directory traversal on both Windows and Linux.
- ✅ Every chunk from a remote peer must pass BLAKE3 verification before being stored or served.

---

## 3. Project File Map

```
p2pstud/
├── AGENT_PLAN.md                  ← This file (update progress tracker here)
├── Cargo.toml                     ← Workspace root
├── Cargo.lock
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs                ← Tauri app entry, registers IPC commands
│   │   ├── lib.rs
│   │   ├── p2p/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs          ← Iroh node lifecycle manager
│   │   │   ├── relay.rs           ← Relay/DERP config helpers
│   │   │   └── identity.rs        ← SecretKey persistence (dirs::data_dir)
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── db.rs              ← SQLite connection pool + migrations
│   │   │   ├── blob_tracker.rs    ← CRUD on local_blobs table
│   │   │   └── eviction.rs        ← LRU eviction background worker
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── storage_cmds.rs    ← get_storage_stats, set_cache_limit
│   │       ├── file_cmds.rs       ← pin_file, unpin_file, read_file_stream
│   │       └── peer_cmds.rs       ← list_peers, connection status
│   └── bin/
│       └── anchor.rs              ← Headless anchor daemon binary
├── src/                           ← Frontend source
│   ├── main.tsx (or main.ts)
│   ├── App.tsx
│   ├── components/
│   │   ├── Explorer/
│   │   │   ├── FileTree.tsx
│   │   │   ├── FileRow.tsx        ← Sync status badges
│   │   │   └── Breadcrumbs.tsx
│   │   ├── Viewers/
│   │   │   ├── PdfViewer.tsx
│   │   │   └── MarkdownEditor.tsx
│   │   ├── Settings/
│   │   │   └── CacheSlider.tsx
│   │   └── Footer/
│   │       └── TelemetryBar.tsx
│   └── hooks/
│       ├── useStorageStats.ts
│       └── usePeerStatus.ts
├── docker/                        ← Optional. Primary deliverable is the binary.
│   ├── Dockerfile                 ← Single image: relay + storage node combined
│   ├── docker-compose.yml         ← One-command VPS deploy (single container)
│   └── .env.example               ← Documents required env vars
├── .github/
│   └── workflows/
│       ├── build.yml              ← Cross-compile Windows + Linux
│       └── test.yml
└── README.md
```

---

## 4. Core Subsystems

### Subsystem A: Embedded Iroh Node & Campus Wi-Fi Traversal

**Assigned subagent:** `RustBackendAgent`

#### A1 — Node Initialization (`p2p/identity.rs` + `p2p/engine.rs`)

```rust
// Pseudocode outline — agent must implement fully
async fn load_or_create_secret_key(data_dir: &Path) -> anyhow::Result<SecretKey> {
    let key_path = data_dir.join("iroh_secret.key");
    if key_path.exists() {
        // read and deserialize existing key
    } else {
        let key = SecretKey::generate();
        // serialize and write atomically to key_path
        // set file permissions to owner-read-only (0o600 on Unix)
    }
}

async fn start_iroh_node(secret_key: SecretKey, relay_url: Url) -> anyhow::Result<Node<Store>> {
    Node::builder()
        .secret_key(secret_key)
        .relay_mode(RelayMode::Custom(relay_url))
        .bind()
        .await
}
```

**Key requirements:**
- Store `SecretKey` in `dirs::data_dir().unwrap().join("p2pstud/iroh_secret.key")`.
- File must be created with owner-only permissions (`0o600` on Linux, restricted ACL on Windows).
- Node handle stored in `tauri::State<Arc<Mutex<IrohEngine>>>`.

#### A2 — Campus NAT Traversal (`p2p/relay.rs`)

- Configure `iroh-net` to attempt direct QUIC first, fall back to the custom relay.
- Relay URL configurable via `tauri.conf.json` or a user settings JSON file:
  ```
  relay_url = "https://relay.yourcampus.org:443"
  ```
- When `MappingVariesByDestIP == true` (symmetric NAT detected), Iroh routes all traffic through the relay automatically — no extra code needed, but **verify this in integration tests** by simulating symmetric NAT with `iptables` or mocking.

---

### Subsystem B: Cooperative Cache & Quota Controller

**Assigned subagent:** `RustBackendAgent`

#### B1 — SQLite Schema (`storage/db.rs`)

```sql
-- Migration 0001
CREATE TABLE IF NOT EXISTS local_blobs (
    hash             TEXT    PRIMARY KEY,
    size_bytes       INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL,  -- Unix timestamp (seconds)
    is_pinned        BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_last_accessed ON local_blobs(last_accessed_at);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings(key, value) VALUES ('cache_limit_bytes', '5368709120'); -- 5 GB
INSERT OR IGNORE INTO settings(key, value) VALUES ('full_mirror', 'false');
```

#### B2 — LRU Eviction Worker (`storage/eviction.rs`)

**Algorithm:**
1. Every 60 seconds (and on every blob write), check total size of unpinned blobs.
2. If total > 90% of quota → evict oldest unpinned blobs until total < 80% of quota.
3. For each evicted blob: delete from `iroh-blobs` store AND delete row from `local_blobs`.
4. If `full_mirror = true`, skip eviction entirely.

**Unit test requirements (TestingAgent):**
- Insert N blobs exceeding the quota; assert eviction removes oldest-first.
- Assert pinned blobs are never evicted regardless of age.
- Assert the eviction target is 80% (not 90%) after a single pass.

#### B3 — Tauri IPC Commands (`commands/storage_cmds.rs`)

```rust
#[tauri::command]
async fn get_storage_stats(state: State<'_, AppState>) -> Result<StorageStats, String>;
// Returns: { used_bytes, quota_bytes, pinned_bytes, peer_count }

#[tauri::command]
async fn set_cache_limit(bytes: Option<u64>, full_mirror: bool, state: State<'_, AppState>) -> Result<(), String>;
// bytes = None means full mirror mode

#[tauri::command]
async fn pin_file(hash: String, state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn unpin_file(hash: String, state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn read_file_stream(hash: String, app: AppHandle, state: State<'_, AppState>) -> Result<Vec<u8>, String>;
// For large files: emit chunks via Tauri events instead of returning Vec<u8> in one shot
```

---

### Subsystem C: Headless VPS Anchor Node

**Assigned subagent:** `AnchorDaemonAgent`

**File:** `src-tauri/src/bin/anchor.rs`

```
USAGE: anchor [OPTIONS]

OPTIONS:
  --data-dir <PATH>          Data directory for blobs and secret key [default: /var/lib/p2pstud]
  --relay-port <PORT>        Port to bind the relay server [default: 443]
  --cert-dir <PATH>          Directory containing TLS cert/key (Let's Encrypt) [default: /etc/letsencrypt/live/relay.yourcampus.org]
  --max-session-bps <N>      Per-session egress cap in bytes/sec [default: 153600 = 150 KB/s]
  --max-aggregate-bps <N>    Aggregate egress cap in bytes/sec [default: 2097152 = 2 MB/s]
  --workspace-id <ID>        Workspace namespace ID to mirror (required)
  --no-relay                 Run as storage-only node (no relay server) — used for headless Docker node variant
```

**Implementation requirements:**
1. Load or generate `SecretKey` from `--data-dir` (same logic as client, reuse `p2p/identity.rs`).
2. Start Iroh node with eviction disabled (full mirror mode always on).
3. Unless `--no-relay` is set, start an `iroh-net` Relay server on `--relay-port` with TLS certs from `--cert-dir`.
4. Implement a token-bucket rate limiter wrapping blob egress:
   - Per-session bucket refills at `--max-session-bps` tokens/sec.
   - Global aggregate bucket refills at `--max-aggregate-bps` tokens/sec.
   - When either bucket is empty, yield with `tokio::time::sleep` before sending the next chunk.

---

### Subsystem C2: Docker Packaging (Optional)

**Assigned subagent:** `AnchorDaemonAgent`

> The `anchor` binary is the **primary deployment artifact**. It runs natively on any Linux server with `./anchor --data-dir /var/lib/p2pstud --relay-port 443 ...`. Docker is a convenience wrapper — not a hard requirement — but if packaged, a **single container must be sufficient** (relay + storage in one image; no multi-container orchestration required).

#### Deployment modes — from simplest to most structured:

| Mode | How | When to use |
|---|---|---|
| **Native binary** | `./anchor [flags]` | Preferred for experienced admins or systemd setups |
| **Single Docker container** | `docker run p2pstud-anchor [flags]` | Quickest VPS setup, no compose needed |
| **Docker Compose (single service)** | `docker compose up -d` | Adds persistent volume + auto-restart without complexity |

#### `docker/Dockerfile` — one image, does everything

```dockerfile
FROM rust:1.82-slim-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src-tauri/ src-tauri/
# Only build the headless anchor binary — not the Tauri desktop app
RUN cargo build --release --bin anchor

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/anchor /usr/local/bin/anchor
VOLUME ["/var/lib/p2pstud", "/etc/letsencrypt"]
EXPOSE 443/tcp 443/udp 80/tcp
ENTRYPOINT ["anchor"]
# Default flags; override via `docker run` args or docker-compose `command:`
CMD ["--data-dir", "/var/lib/p2pstud", "--relay-port", "443"]
```

The `--no-relay` flag on the binary lets the same image act as a **pure storage node** (no relay server started), so one Dockerfile covers both VPS and headless-node-only use cases.

#### `docker/docker-compose.yml` — single service, single container

```yaml
version: "3.9"
services:
  p2pstud:
    build:
      context: ..
      dockerfile: docker/Dockerfile
    image: p2pstud:latest
    restart: unless-stopped
    network_mode: host          # best QUIC/UDP performance; avoids NAT inside container
    volumes:
      - p2pstud_data:/var/lib/p2pstud
      - /etc/letsencrypt:/etc/letsencrypt:ro
    env_file: .env
    command: >
      --data-dir /var/lib/p2pstud
      --relay-port 443
      --cert-dir /etc/letsencrypt/live/${RELAY_DOMAIN}
      --workspace-id ${WORKSPACE_ID}
      --max-session-bps ${MAX_SESSION_BPS:-153600}
      --max-aggregate-bps ${MAX_AGGREGATE_BPS:-2097152}
    healthcheck:
      test: ["CMD", "anchor", "--health-check"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  p2pstud_data:
    driver: local
```

#### `docker/.env.example`

```bash
# Copy to .env before running docker compose
RELAY_DOMAIN=relay.yourcampus.org
WORKSPACE_ID=                   # Required: get from workspace admin
MAX_SESSION_BPS=153600          # 150 KB/s per peer
MAX_AGGREGATE_BPS=2097152       # 2 MB/s total outbound
```

#### Native binary with systemd (no Docker)

```ini
# /etc/systemd/system/p2pstud-anchor.service
[Unit]
Description=P2P Campus Workspace Anchor Node
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/anchor \
  --data-dir /var/lib/p2pstud \
  --relay-port 443 \
  --cert-dir /etc/letsencrypt/live/relay.yourcampus.org \
  --workspace-id YOUR_WORKSPACE_ID
Restart=on-failure
User=p2pstud
WorkingDirectory=/var/lib/p2pstud

[Install]
WantedBy=multi-user.target
```

---

### Subsystem D: Student Workspace Frontend

**Assigned subagent:** `FrontendAgent`

#### D1 — Virtual File Explorer

File sync status badges (CSS classes or Lucide icon variants):
| State | Badge | Color |
|---|---|---|
| Cloud Only | `☁ Cloud Only` | Gray |
| Cached / Seeding | `⬆ Seeding` | Blue |
| Pinned | `📌 Pinned` | Green |
| Downloading | `⬇ X%` | Orange (progress) |

- Breadcrumb navigation, click-to-open directories.
- Drag-and-drop upload: drop files → call Tauri command to add blobs → update explorer.
- Right-click context menu: **Pin / Unpin**, **Download**, **Preview**, **Copy Hash**.

#### D2 — PDF Viewer (`components/Viewers/PdfViewer.tsx`)

- Use `pdfjs-dist` with worker bundled via Vite.
- Stream bytes from `read_file_stream` Tauri command (for large PDFs: receive via event stream, assemble `Uint8Array`, pass to `pdfjsLib.getDocument({ data })`).
- Render page-by-page with virtual scrolling for performance.

#### D3 — Markdown Editor (`components/Viewers/MarkdownEditor.tsx`)

- Left pane: Monaco or TipTap editor.
- Right pane: live HTML preview (use `marked` or `remark` for rendering).
- Auto-save: debounce 1 second → serialize to blob → call Tauri to update the blob hash in the workspace index.

#### D4 — Telemetry Footer (`components/Footer/TelemetryBar.tsx`)

```
[ ● Connected ]  Peers: 3  |  Cache: 2.3 / 5.0 GB  |  ↑ 1.2 MB/s  ↓ 4.5 MB/s
```

- Polls `get_storage_stats()` every 5 seconds via Tauri IPC.
- Uses Tauri event listener for real-time peer connection/disconnection events.

#### D5 — Settings Panel (`components/Settings/CacheSlider.tsx`)

- Slider: 3 GB → 50 GB, with a "Full Mirror" toggle at the end.
- On change: call `set_cache_limit(bytes, full_mirror)` → immediately triggers eviction check.

---

## 5. Security Requirements

> These are **non-negotiable**. The security audit step (Step 17) must verify each point.

1. **Chunk integrity:** Every blob received from a remote peer passes BLAKE3 verification inside `iroh-blobs` before being written to disk. This is built-in to Iroh — ensure it is not bypassed.
2. **Path sanitization:** All user-supplied file paths or names must be canonicalized and checked to remain within the workspace data directory:
   ```rust
   fn safe_join(base: &Path, user_input: &str) -> anyhow::Result<PathBuf> {
       let joined = base.join(user_input);
       let canonical = joined.canonicalize()?;
       if !canonical.starts_with(base) {
           anyhow::bail!("Path traversal attempt detected");
       }
       Ok(canonical)
   }
   ```
3. **No open RPC:** The Iroh control plane runs entirely in-process. No HTTP or RPC ports are bound to `0.0.0.0` on student machines.
4. **SecretKey file permissions:** `0o600` on Linux. On Windows, remove the `Everyone` ACE and grant read access only to the current user account via `std::os::windows::fs::OpenOptionsExt`.
5. **Relay TLS:** The anchor node relay must present a valid TLS certificate. Clients verify the relay's certificate; pinned or TOFU (Trust-on-First-Use) strategy acceptable for MVP.

---

## 6. Testing Strategy

**Assigned subagent:** `TestingAgent`

| Test Type | Coverage Target | Tool |
|---|---|---|
| Unit tests | Eviction algorithm, SQLite CRUD, path sanitization | `cargo test` |
| Integration | Two local Iroh nodes transfer a blob, verify BLAKE3 | `cargo test --features integration` |
| Frontend | Component rendering, IPC mock responses | Vitest + Testing Library |
| End-to-end | Full app launch, file upload, download on second instance | Tauri driver / Playwright |

**Minimum before shipping:**
- [ ] Eviction unit tests pass (Step 7).
- [ ] Path traversal attempts return `Err`, never succeed.
- [ ] Two-node blob transfer integration test passes locally.

---

## 7. CI/CD (`.github/workflows/`)

**`build.yml`** — triggers on push to `main` and PRs:
```yaml
strategy:
  matrix:
    target:
      - x86_64-pc-windows-msvc
      - x86_64-unknown-linux-gnu
```
- Cargo build + test for both targets.
- Frontend `npm run build` check.
- Upload installers as GitHub release artifacts on tagged commits.

**`test.yml`** — triggers on all PRs:
- `cargo test --all`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- `npm run test` (Vitest)

---

## 8. Deployment: Anchor Node on VPS

**One-command deploy:**
```bash
git clone https://github.com/yourorg/p2pstud
cd p2pstud/docker
# Edit .env with RELAY_DOMAIN, WORKSPACE_ID
docker compose up -d
```

**Let's Encrypt certificate (certbot, run once before launching):**
```bash
certbot certonly --standalone -d relay.yourcampus.org --agree-tos -m admin@yourcampus.org
```

**`docker-compose.yml`** must:
- Mount `/etc/letsencrypt` read-only into the container.
- Mount a persistent volume for blob storage.
- Expose ports 80 (certbot renewal), 443/tcp, 443/udp.
- Set `restart: unless-stopped`.

---

## 9. Milestones & Acceptance Criteria

| Milestone | Acceptance Criteria |
|---|---|
| **M1: Networking** | Two app instances on the same LAN discover each other and exchange a test blob. |
| **M2: Campus Wi-Fi** | Two instances behind Eduroam (or simulated symmetric NAT) exchange a blob via the relay. |
| **M3: Cache Management** | Adding blobs beyond the quota automatically evicts oldest-first; pinned blobs survive. |
| **M4: UI Complete** | File explorer, PDF viewer, Markdown editor, and telemetry footer all functional. |
| **M5: Anchor Online** | VPS anchor holds all blobs; offline student can retrieve files at ≤ 150 KB/s. |
| **M6: Packaged** | `tauri build` produces working `.msi` (Windows) and `.deb` / `.AppImage` (Linux) installers. |

---

## 10. Known Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Iroh API churn (semver pre-1.0) | Pin all `iroh*` crates to exact versions in `Cargo.lock`; review changelog before upgrading. |
| `iroh-blobs` streaming API unfamiliar | Read `iroh-blobs` examples in the official repo before implementing `read_file_stream`. |
| Campus firewall blocking QUIC UDP entirely | Iroh falls back to relay over TCP 443; test this path explicitly. |
| Large PDF streaming (> 100 MB) | Use Tauri event-based chunking, not single IPC return value. |
| Windows ACL for SecretKey file | Use `windows-acl` crate or PowerShell `icacls`; test on a standard user account (non-admin). |
| VPS bandwidth exhaustion | Token-bucket rate limiter in anchor daemon; set VPS monthly bandwidth alert at 80%. |

---

*Last updated: 2026-11-02 — initial plan creation.*


