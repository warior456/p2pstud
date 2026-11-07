# P2P Campus Workspace ΓÇö Agent Build Plan

> **For the executing agent:** Read this file at the start of every session. Update the `## Progress Tracker` section after completing each task. Never skip the tracker update ΓÇö it is the sole source of truth about where work left off.

> **Companion files ΓÇö read these too before starting work:**
> - `REQUIREMENTS.md` ΓÇö full requirements table, dependency versions, design decisions log, open questions
> - `AGENTS.md` ΓÇö subagent roles, file ownership, startup checklists, completion criteria

---

## Agent Orchestration Guidelines

### How to Use This Plan

1. **Start every session** by reading this file top-to-bottom to orient yourself on current state.
2. **Track progress** by checking off tasks in the Progress Tracker and updating the `Last Completed` timestamp and session note.
3. **Use subagents liberally** for parallelizable work (see Subagent Strategy below).
4. **Version Bumping on Fixes**: Every change that fixes something must increase the latest `x.x.this` (patch) of the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
5. **After each major milestone**, run the full test suite and commit with a descriptive message.
6. **If blocked**, write a `BLOCKED:` note in the relevant tracker row before stopping, so the next session knows what to investigate.

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
- Mark completed items Γ£à
- Mark in-progress items ≡ƒöä
- Mark blocked items Γ¥î with a `BLOCKED:` note
- Update `**Last session:**` at the top of the tracker

---

## Progress Tracker

> **Last session:** 2026-11-07
> **Last completed task:** Step 68 completed: Live Online Seed Verification On App Close & Swarm Catalog Refresh (v0.1.38).
> **Next action:** Production release distribution and testing.

| # | Step | Status | Notes |
|---|---|---|---|
| 1 | Tauri v2 scaffold + Cargo workspace | Γ£à | Cargo workspace, Tauri config, package.json set up |
| 2 | Iroh node engine (`p2p/engine.rs`) | Γ£à | Implemented with QUIC endpoint, relay fallback, blob store |
| 3 | SecretKey persistence & node startup | Γ£à | Implemented in `p2p/identity.rs` with atomic write & OS file restrictions |
| 4 | Tauri IPC command surface | Γ£à | `storage_cmds`, `file_cmds`, `peer_cmds` implemented in `commands/` |
| 5 | SQLite schema + blob tracker | Γ£à | `storage/db.rs` (WAL, migrations) and `storage/blob_tracker.rs` implemented |
| 6 | LRU eviction background worker | Γ£à | `storage/eviction.rs` implemented (90% high mark, 80% low mark) |
| 7 | Pin/unpin logic + tests | Γ£à | Pinned blob survival, oldest-first eviction unit tests implemented |
| 8 | Frontend scaffold (Vite + React/Svelte + Tailwind) | Γ£à | React 18 + Vite + TailwindCSS + Lucide Icons configured |
| 9 | Virtual file explorer UI | Γ£à | `FileTree.tsx`, `FileRow.tsx`, `Breadcrumbs.tsx` with 4 sync-state badges |
| 10 | PDF viewer integration | Γ£à | `PdfViewer.tsx` implemented with `pdfjs-dist` & canvas rendering |
| 11 | Markdown editor + split-pane preview | Γ£à | `MarkdownEditor.tsx` with live marked HTML preview and debounced auto-save |
| 12 | Network telemetry footer | Γ£à | `TelemetryBar.tsx` polling storage stats & peer connection status |
| 13 | Anchor daemon binary (`src-tauri/src/bin/anchor.rs`) | Γ£à | Headless binary with Clap CLI, TLS cert support, `--no-relay` flag |
| 14 | Token-bucket rate limiter on anchor | Γ£à | Session & aggregate token-bucket throttling implemented |
| 15 | Docker packaging (optional) ΓÇö single `Dockerfile` + `docker-compose.yml` | Γ£à | `docker/Dockerfile`, `docker-compose.yml`, `.env.example`, `p2pstud-anchor.service` |
| 16 | End-to-end connectivity test (2 local nodes) | Γ£à | Feature-gated integration test passing (`two_nodes_transfer_blob`) |
| 17 | Security audit (path sanitization, no open RPC) | Γ£à | `path_utils.rs` (`safe_join`), owner-only key permissions, in-process IPC |
| 18 | CI workflow (GitHub Actions) | Γ£à | Multi-platform build matrix in `.github/workflows/build.yml` (Windows, Linux, macOS ARM64 & x86_64) |
| 19 | README + deployment guide | Γ£à | Comprehensive `README.md` and `.gitignore` created |
| 20 | Production Readiness Polish & Background Tray | Γ£à | System tray icon, window close interception, network & peer configuration, purge cache, installer & portable ZIP |
| 21 | Cross-Platform Linux & Windows Parity | Γ£à | `xdg-open` on Linux, `0o600` secret key permissions, bundled SQLite, `.deb` & `.AppImage` bundle config in Tauri, CI matrix for Windows & Ubuntu |
| 22 | Multi-Layer Discovery & Smart Peer Dialing | Γ£à | `LocalSwarmDiscovery` on LAN, `n0_dns` and `pkarr_dht` on WAN, auto background discovery connect, `NodeTicket` parsing |
| 23 | Workspace Catalog Synchronization & Remote Fetch | Γ£à | Real-time QUIC catalog exchange (`/p2pstud/sync/1`), broadcast on upload/delete, on-demand remote blob download |
| 24 | Active Swarm Hosting (~80%) & Orphaned File Management | Γ£à | Worker actively fills cache to 80% to seed course files; orphaned file force-cleanup for departed owners |
| 25 | Live Bandwidth Telemetry & Community Deletion Consensus Voting | Γ£à | Real-time byte-rate tracker replaces static mock speeds; community vote-to-delete with 3-vote quorum |
| 26 | Upload Queue Visual Indicator & Smart Least-Seeds Cache Auto-Fill | Γ£à | Live upload queue drawer & file row progress; smart auto-fill prioritizing lowest seed counts |
| 27 | Hierarchical Folder Support & Organization | Γ£à | Virtual folder tree, directory navigation, "New Folder", folder-aware uploads & breadcrumb deep-linking |
| 28 | File Moving & Sole Active Seed Disk Eviction Protection | Γ£à | Drag-and-drop & context file moving with QUIC sync; prevent disk eviction if local node is sole seed |
| 29 | Anti-Delete (Keep) Voting, 86% Consensus, 24h Grace Period & Offline Bundling | Γ£à | Non-owners blocked from unilateral delete; Keep defense voting; 86% consensus ratio threshold; 24h grace period warning banner; offline-embedded release build |
| 30 | Single-Instance Integration & Multi-Instance WebView2 Crash Fix | Γ£à | Integrated `tauri-plugin-single-instance` to prevent 0x800700AA crash; safe SQLite forward-migration bounds check |
| 31 | Security Audit & Comprehensive Hardening | Γ£à | Virtual path sanitization (`sanitize_virtual_path`), DOMPurify XSS mitigation in Markdown preview, authenticated QUIC vote verification, owner-only file moving, null-byte rejection |
| 32 | P2P Swarm Network Secret & Dev Mode ZIP Import | Γ£à | Swarm passphrase derives dynamic QUIC ALPN isolating private student networks; Zip-Slip safe bulk ZIP archive import in dev mode |
| 33 | Campus Wi-Fi Detection & Eduroam Stealth Mode | ✅ | Auto-detect `eduroam`/`campusroam`, disable DHT/mDNS on campus, port 443 camouflage, LEDBAT & lecture throttling |
| 34 | First-Run Onboarding Wizard & Connection Doctor | ✅ | Simplified vs Advanced setup, modular credentials, interactive P2P network diagnostics test suite |
| 35 | Smart Scarcity Replication & SSD Wear Defense | ✅ | Auto-download 1-seed files, abundance-first eviction ($\ge 3$), 24-48h dwell time, daily write budget, in-memory buffer |
| 36 | Sole Seed Presence & Desktop Alerts | ✅ | Floating warning banner, flashing taskbar notification, highlighted tray state when hosting unique files |
| 37 | Strict Immutability & Path Collision Resolution | ✅ | Strict absolute path lock, timestamp-based offline conflict resolution, executable warning modal, cross-platform normalization |
| 38 | Study Ratings, Empty Folder Rules & Emergency Quarantine | ✅ | Star bookmarks, up/downvote quality signals, negative seeding demotion, 5-report emergency quarantine (1-week hold), empty-only folder deletion |
| 39 | Transfer Resumption & Enhanced Offline Mode | ✅ | Persistent SQLite transfer resume queue, offline header banner, 'Available Offline' filter toggle |
| 40 | Workspace Backup, Folder ZIP Export & Updates | ✅ | Settings full app backup, folder ZIP export with peer download prompt, startup/shutdown Tauri auto-updater |
| 41 | Bilateral Seeding Receipts & Lifecycle Pruning | ✅ | Hash-verified transfer receipts for Sybil-resistant voting, 5-year seeding deprioritization, 2-year dead file purge |
| 42 | Multi-GB Streaming ZIP Import, Multi-File Selection & Cross-Platform Autostart | ✅ | Streaming disk BufReader & `add_from_path` for multi-GB ZIPs (`semester1.zip`), native `rfd` picker, batch multi-file delete/move/download, cross-platform boot autostart & custom cache location |
| 43 | Dependabot Alert Fix (CVE-2026-115905, zip 9.3.0) & Lockfile Tracking | ✅ | Upgraded `zip` crate to 9.3.0 with Cow string unwrapping, un-ignored and committed `Cargo.lock` so Dependabot can audit installed crates |
| 44 | Import Bandwidth Isolation, Dedicated Importing Banner & Unified Network Tab | ✅ | Excluded local disk imports from P2P network speed metrics, added dedicated `ImportingBanner` ("importing balk") with animated progress & disk speed, added separate footer import badge, and moved network sub-menu from "Cache & Quota" into main "Network & Swarm" tab |
| 45 | GitHub Releases Auto-Updater, Quit Intercept Modal, Swarm Median Version & Dependabot Security Upgrades | ✅ | Fully implemented live GitHub Releases updater targeting `https://github.com/warior456/p2pstud/releases` with real installer download and execution; fixed Sole-Seed & Update Quit Intercept Modal (R16.2/3, R23.3) ensuring the confirmation modal is never bypassed; calculated and displayed Swarm Median Version in telemetry bar and settings; updated `build.yml` CI workflow to automatically publish release builds on tag or workflow_dispatch; updated `zip` crate to `v9.4.0` resolving Dependabot CVE-2026-115905, updated `Cargo.lock` and synced `src-tauri/Cargo.lock`. Version bumped to 0.1.12. |
| 46 | Wi-Fi Swarm Reliability, Connection Doctor & Offline Banner Fix | ✅ | Fixed false "Offline" banner when peer_count=0; prevented Connection Doctor freeze via non-blocking DNS and safety timeouts; used `RelayMode::Default` for closest optimal relay; emitted `iroh-node-ready` for instant NodeId resolution; added SSID cache invalidation on reconnect. Version bumped to 0.1.13. |
| 47 | Multi-Network DB Isolation, System Tray Exit Modal, Live Offline Detection, EU Relay & Input Fix | ✅ | Tray exit intercept reliably triggers quit modal; `./target` disk cleanup; default EU relay with EU/US/Global presets; live relay reachability detection for offline banner; settings input debounce fix; isolated per-network databases with cross-network file transfer under strict seedability guarantee. Version bumped to 0.1.14. |
| 48 | EU Relay Standardization, Restored Window [X] Minimize, Streamlined Sole-Seed Quit, Connection Doctor & DB Delete | ✅ | Western Europe EU relay lock; ToS verification; TCP 443 live reachability probe; direct quit on non-sole-seed and warning popup on sole seed; restored [X] minimizing to tray; Connection Doctor concrete results and Blocked rendering; Clear Database & Delete Database buttons. Version bumped to 0.1.15. |
| 49 | Offline-First Resilience: Preloaded BlobStore, Disk NodeId Ownership & Offline File Reading | ✅ | Preloaded local BlobStore in AppState on boot; offline cache recognition for 3GB cached blobs in `list_files`; direct offline byte reading in `read_file_bytes`; persistent disk Ed25519 identity fallback for `get_node_id` & file ownership; endpoint fallback & retry loop. Version bumped to 0.1.16. |
| 50 | Seamless In-Place NSIS Auto-Updating (No Uninstall/Reinstall Prompts) | ✅ | Configured NSIS `installMode: "currentUser"` & `allowDowngrades: true`; removed `msi` from bundle targets; prioritized `-setup.exe` in asset matching; detached windowless updater runner waits for process exit, runs silent `/S` in-place upgrade, and relaunches the upgraded application. Version bumped to 0.1.17. |
| 51 | Automated HTTPS Swarm Rendezvous (Zero-Config Peer Discovery) | ✅ | Implemented `p2p/rendezvous.rs` topic-based discovery over HTTPS port 443; enables zero-config peer pairing on client-isolated Wi-Fi and firewalled campus networks without DHT or manual ticket exchange; added manual discovery trigger and UI status in Network Settings. Version bumped to 0.1.19. |
| 52 | GitHub Updater Rate-Limit Resilience | ✅ | Added HTML redirect fallback on `releases/latest` to resolve latest release and assets when `api.github.com` unauthenticated IP rate limit (HTTP 403) is exhausted on shared campus networks. Version bumped to 0.1.20. |
| 53 | Single-Store Architecture, Concurrent Redb Lock Fix & Ready-Polling Discovery | ✅ | Unified BlobStore initialization to prevent concurrent redb open conflicts between preload and IrohEngine; added graceful readiness polling to `get_node_ticket`, `get_node_addr`, and `trigger_swarm_discovery`; hooked `iroh-node-ready` in Network Settings; bumped version to 0.1.24. |
| 54 | Security Audit Remediations & Clippy Cleanup (VULN-01 to VULN-14) | ✅ | Remediated all 14 security vulnerabilities from `SECURITY_AUDIT_REPORT.md`, resolved 14 clippy compiler warnings with `-D warnings`, eliminated eviction deadlock, enforced updater cryptographic verification & path confinement, and bumped version to 0.1.25. |
| 55 | Specification & Architecture Drift Resolution (`egm-resolve`) | ✅ | Resolved all 13 drift findings from 2026-11-07 audit report; enshrined zero-domain, light-server VPS auth spec in `KULEUVEN_AUTH_SPEC.md`; staged rate limiter docs; synchronized R14-R24 & R7 dependency matrix; bumped version to 0.1.26. |
| 56 | Anchor Connection Discoverability & Desktop NSIS Installer Package | ✅ | Clarified remote peer / anchor node connection form and bootstrap peer persistence guidance in `NetworkSettings.tsx`; successfully built release NSIS desktop installer bundle (`p2pstud_0.1.26_x64-setup.exe`, 13.82 MiB); bumped version to 0.1.27. |
| 57 | Zero-Domain Light VPS Anchor Defaults & Deployment Documentation | ✅ | Standardized `docker/Dockerfile`, `docker-compose.yml`, `docker/.env.example`, and `docker/p2pstud-anchor.service` to default to zero-domain `--no-relay` storage mode with public EU relay fallback; documented light VPS setup in `README.md`; bumped version to 0.1.28. |
| 58 | Windows Release NSIS Installer Packaging (v0.1.28) | ✅ | Successfully compiled release binary and packaged NSIS desktop installer bundle (`p2pstud_0.1.28_x64-setup.exe`, 13.82 MiB) with all 37 Vitest tests passing. |
| 59 | Dependabot Alert Fix (zip 9.4.1), Lockfile Sync & Live Updater Progress (v0.1.29) | ✅ | Upgraded `zip` crate to `9.4.1`, synced root and `src-tauri` Cargo.lock, deduplicated `dependabot.yml` cargo config, added live update download progress streaming and foreground setup launcher, prevented abrupt window close in `lib.rs`, and bumped version to 0.1.29. |
| 60 | Windows Release NSIS Installer Packaging (v0.1.29) | ✅ | Successfully compiled optimized release binary and packaged NSIS desktop installer bundle (`p2pstud_0.1.29_x64-setup.exe`, 13.82 MiB) with all 37 Vitest tests passing. |
| 61 | Automated Swarm Discovery, Relay Modernization & Peer Visibility (v0.1.30) | ✅ | Fixed dead Iroh 0.x relay domain to `euc1-1.relay.n0.iroh.link`, enabled `AddrFilter::unfiltered()` for local/direct IP dialing, converted `CatalogSyncHandler` to multiplexed accept bi-stream loop, resolved announce reflection bug, included non-seeding peers in peer counts/listing via `peer_versions` and in-memory active endpoints, augmented bare NodeId connects with active relay, updated DB migration 0016, and passed all integration and unit tests. |
| 62 | NodeId Canonicalization, Disk Blob Auto-Heal & Silent In-Place Binary Updater (v0.1.32) | ✅ | Resolved base32 vs hex node ID mismatch for "You (Owner)" badge; added physical disk blob auto-healing for 3.7GB cached blobs; fixed Sole-Seed banner click-to-filter & interactive Node ID copying; replaced interactive foreground installer with silent in-place binary replacement across Windows, Linux & macOS; exported intermediary standalone binaries in GitHub Actions without duplicate builds. |
| 63 | Importer Progress Bar, Live Disk Speed & Auto-Dismiss Lifecycle (v0.1.33) | ✅ | Created `src-tauri/capabilities/default.json` restoring Tauri v2 `core:event` IPC permissions so frontend receives `import-progress` events; added dual app/window event delivery; implemented rolling local disk speed throughput calculation; guaranteed `status: 'completed'` and 3.5s auto-dismissal in `ImportingBanner.tsx` and `handleExecuteZipUnpack`. |
| 64 | Restored Window Close Button [X] Minimize-to-Tray (v0.1.34) | ✅ | Restored `window.hide()` on `CloseRequested` in `src-tauri/src/lib.rs` resolving regression where clicking window close `[X]` was intercepted without hiding or closing the window; maintains continuous background P2P seeding while tray quit and UI power button handle application termination. |
| 65 | Active Upload Bandwidth Measurement Integration (v0.1.35) | ✅ | Hooked `BandwidthTracker::record_upload` across `upload_file_path`, `upload_file_bytes`, `add_file`, `import_zip_archive_internal`, and QUIC catalog sync streams in `p2p/sync.rs`; added global bandwidth accessors so live upload throughput `↑ X.X MB/s` reflects real upload activity in the telemetry bar. |
| 66 | Local LAN UDP Broadcast Discovery & Multi-Provider Rendezvous Fallback (v0.1.36) | ✅ | Implemented zero-configuration local LAN and loopback peer discovery via UDP broadcast socket on port 27888 with Ed25519 signature verification; added multi-provider HTTPS rendezvous fallback (`ntfy.tedomum.fr`, `ntfy.adminforge.de`, `ntfy.sh`) resolving public `ntfy.sh` 429 quota exhaustion; auto-connects peers immediately even with clean/reset databases. |
| 67 | Cache Size Physical Disk Synchronization & Orphaned Blob Pruning (v0.1.37) | ✅ | Reconciled cache size discrepancy between UI and Windows Explorer by measuring physical blob store directory usage (`calculate_dir_size`); implemented physical disk file deletion (`delete_blob_files_from_disk`) across file deletions, cache purges, LRU eviction, and lifecycle pruning; added automatic orphaned disk blob pruning (`cleanup_orphaned_disk_blobs`) on startup, DB clear, and cache purge. |
| 68 | Live Online Seed Verification On App Close & Swarm Catalog Refresh (v0.1.38) | ✅ | Implemented live concurrent QUIC ping verification (`verify_candidate_remote_seeds`) before exiting from power button or tray icon, ensuring offline peers have their seed records expired immediately so sole-seed protection triggers properly; added `refresh_swarm_catalog` IPC command and wired Refresh button to perform live peer verification, rendezvous polling, and catalog synchronization with spin animation. |

---

## 1. Executive Summary & Objective

Build a lightweight, cross-platform desktop application (Windows + Linux) that enables cooperative P2P academic file sharing across restrictive university networks (e.g., Eduroam).

**Core model ΓÇö "Host-to-Play" cooperative storage:**
- Every student dedicates configurable local disk space (default **5 GB**, toggle to "Full Mirror") to store and seed chunks.
- A cheap cloud VPS (\$3ΓÇô5/month) is the **Anchor Node**: always-on persistence + private relay on TCP 443 + throttled fallback when peers are offline.
- All cryptography, chunk verification, and NAT traversal come from **Iroh** embedded directly in the Tauri binary ΓÇö no custom socket protocols, no rolling crypto.

---

## 2. Technology Stack & Constraints

| Layer | Choice | Constraint |
|---|---|---|
| App shell | Tauri v2 (Rust + WebView) | Idle RAM Γëñ 120 MB |
| P2P identity + transport | `iroh` + `iroh-net` | QUIC; relay fallback on port 443 |
| Content-addressed storage | `iroh-blobs` | BLAKE3 integrity on every chunk |
| Local state | `rusqlite` (SQLite) | Tracks blobs, timestamps, pin state |
| Frontend | React 18 + TailwindCSS + Lucide Icons | Bundled via Vite |
| PDF viewer | `pdfjs-dist` or `@react-pdf-viewer/core` | |
| Markdown editor | `@monaco-editor/react` or Milkdown/TipTap | Split-pane live preview |
| Async runtime | `tokio` (already used by Iroh) | |

**Hard constraints for the agent:**
- Γ¥î Do NOT write custom socket/crypto protocols.
- Γ¥î Do NOT bind unauthenticated HTTP/RPC ports to `0.0.0.0` on student machines.
- Γ£à All file paths must be sanitized against directory traversal on both Windows and Linux.
- Γ£à Every chunk from a remote peer must pass BLAKE3 verification before being stored or served.

---

## 3. Project File Map

```
p2pstud/
├── AGENT_PLAN.md                  → This file (update progress tracker here)
├── Cargo.toml                     → Workspace root
├── Cargo.lock
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs                → Tauri app entry, registers IPC commands
│   │   ├── lib.rs
│   │   ├── p2p/
│   │   │   ├── mod.rs
│   │   │   ├── anchor_voucher.rs  → Attestation vouchers issued by anchors
│   │   │   ├── bandwidth.rs       → Live upload/download rate tracking
│   │   │   ├── engine.rs          → Iroh node lifecycle manager
│   │   │   ├── identity.rs        → SecretKey persistence (dirs::data_dir)
│   │   │   ├── lan_discovery.rs   → Local LAN & loopback UDP broadcast discovery
│   │   │   ├── network_profile.rs → Eduroam SSID detection & campus stealth
│   │   │   ├── relay.rs           → Relay/DERP config helpers
│   │   │   ├── rendezvous.rs      → HTTPS zero-config peer discovery
│   │   │   └── sync.rs            → Peer catalog synchronization
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── blob_tracker.rs    → CRUD on local_blobs table
│   │   │   ├── db.rs              → SQLite connection pool + migrations
│   │   │   ├── eviction.rs        → LRU eviction background worker
│   │   │   ├── hosting_worker.rs  → Swarm auto-fill hosting worker
│   │   │   ├── path_utils.rs      → Cross-platform path sanitization (safe_join)
│   │   │   ├── pruning.rs         → 5-year seeding decay & 2-year dead purge
│   │   │   └── scarcity.rs        → Scarcity-first auto-download & SSD protection
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── backup_cmds.rs     → App data backup, ZIP export & restore
│   │   │   ├── diagnostic_cmds.rs → Connection Doctor diagnostics
│   │   │   ├── file_cmds.rs       → pin_file, unpin_file, read_file_stream
│   │   │   ├── peer_cmds.rs       → list_peers, connection status, rendezvous
│   │   │   └── storage_cmds.rs    → get_storage_stats, set_cache_limit
│   │   └── bin/
│   │       └── anchor.rs          → Headless anchor daemon binary
│   └── tests/
├── src/                           → Frontend source (React 18 + TypeScript)
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── ImportingBanner.tsx
│   │   ├── OfflineBanner.tsx
│   │   ├── SoleSeedBanner.tsx
│   │   ├── Explorer/
│   │   │   ├── Breadcrumbs.tsx
│   │   │   ├── FileRow.tsx
│   │   │   ├── FileTree.tsx
│   │   │   ├── UploadQueueDrawer.tsx
│   │   │   └── ZipImportModal.tsx
│   │   ├── Onboarding/
│   │   │   ├── ConnectionDoctor.tsx
│   │   │   ├── CredentialsStep.tsx
│   │   │   ├── OnboardingAdvanced.tsx
│   │   │   ├── OnboardingSimple.tsx
│   │   │   └── OnboardingWizard.tsx
│   │   ├── Settings/
│   │   │   ├── BackupSettings.tsx
│   │   │   ├── CacheSlider.tsx
│   │   │   └── NetworkSettings.tsx
│   │   ├── Viewers/
│   │   │   ├── MarkdownEditor.tsx
│   │   │   └── PdfViewer.tsx
│   │   └── Footer/
│   │       └── TelemetryBar.tsx
│   └── hooks/
│       ├── usePeerStatus.ts
│       └── useStorageStats.ts
├── docker/
│   ├── Dockerfile
│   ├── docker-compose.yml
│   ├── p2pstud-anchor.service
│   └── .env.example
├── .github/
│   └── workflows/
│       └── build.yml
└── README.md
```

---

## 4. Core Subsystems

### Subsystem A: Embedded Iroh Node & Campus Wi-Fi Traversal

**Assigned subagent:** `RustBackendAgent`

#### A1 ΓÇö Node Initialization (`p2p/identity.rs` + `p2p/engine.rs`)

```rust
// Pseudocode outline ΓÇö agent must implement fully
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

#### A2 ΓÇö Campus NAT Traversal (`p2p/relay.rs`)

- Configure `iroh-net` to attempt direct QUIC first, fall back to the custom relay.
- Relay URL configurable via `tauri.conf.json` or a user settings JSON file:
  ```
  relay_url = "https://relay.yourcampus.org:443"
  ```
- When `MappingVariesByDestIP == true` (symmetric NAT detected), Iroh routes all traffic through the relay automatically ΓÇö no extra code needed, but **verify this in integration tests** by simulating symmetric NAT with `iptables` or mocking.

---

### Subsystem B: Cooperative Cache & Quota Controller

**Assigned subagent:** `RustBackendAgent`

#### B1 ΓÇö SQLite Schema (`storage/db.rs`)

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

#### B2 ΓÇö LRU Eviction Worker (`storage/eviction.rs`)

**Algorithm:**
1. Every 60 seconds (and on every blob write), check total size of unpinned blobs.
2. If total > 90% of quota ΓåÆ evict oldest unpinned blobs until total < 80% of quota.
3. For each evicted blob: delete from `iroh-blobs` store AND delete row from `local_blobs`.
4. If `full_mirror = true`, skip eviction entirely.

**Unit test requirements (TestingAgent):**
- Insert N blobs exceeding the quota; assert eviction removes oldest-first.
- Assert pinned blobs are never evicted regardless of age.
- Assert the eviction target is 80% (not 90%) after a single pass.

#### B3 ΓÇö Tauri IPC Commands (`commands/storage_cmds.rs`)

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
  --no-relay                 Run as storage-only node (no relay server) ΓÇö used for headless Docker node variant
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

> The `anchor` binary is the **primary deployment artifact**. It runs natively on any Linux server with `./anchor --data-dir /var/lib/p2pstud --relay-port 443 ...`. Docker is a convenience wrapper ΓÇö not a hard requirement ΓÇö but if packaged, a **single container must be sufficient** (relay + storage in one image; no multi-container orchestration required).

#### Deployment modes ΓÇö from simplest to most structured:

| Mode | How | When to use |
|---|---|---|
| **Native binary** | `./anchor [flags]` | Preferred for experienced admins or systemd setups |
| **Single Docker container** | `docker run p2pstud-anchor [flags]` | Quickest VPS setup, no compose needed |
| **Docker Compose (single service)** | `docker compose up -d` | Adds persistent volume + auto-restart without complexity |

#### `docker/Dockerfile` ΓÇö one image, does everything

```dockerfile
FROM rust:1.82-slim-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src-tauri/ src-tauri/
# Only build the headless anchor binary ΓÇö not the Tauri desktop app
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

#### `docker/docker-compose.yml` ΓÇö single service, single container

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

#### D1 ΓÇö Virtual File Explorer

File sync status badges (CSS classes or Lucide icon variants):
| State | Badge | Color |
|---|---|---|
| Cloud Only | `Γÿü Cloud Only` | Gray |
| Cached / Seeding | `Γ¼å Seeding` | Blue |
| Pinned | `≡ƒôî Pinned` | Green |
| Downloading | `Γ¼ç X%` | Orange (progress) |

- Breadcrumb navigation, click-to-open directories.
- Drag-and-drop upload: drop files ΓåÆ call Tauri command to add blobs ΓåÆ update explorer.
- Right-click context menu: **Pin / Unpin**, **Download**, **Preview**, **Copy Hash**.

#### D2 ΓÇö PDF Viewer (`components/Viewers/PdfViewer.tsx`)

- Use `pdfjs-dist` with worker bundled via Vite.
- Stream bytes from `read_file_stream` Tauri command (for large PDFs: receive via event stream, assemble `Uint8Array`, pass to `pdfjsLib.getDocument({ data })`).
- Render page-by-page with virtual scrolling for performance.

#### D3 ΓÇö Markdown Editor (`components/Viewers/MarkdownEditor.tsx`)

- Left pane: Monaco or TipTap editor.
- Right pane: live HTML preview (use `marked` or `remark` for rendering).
- Auto-save: debounce 1 second ΓåÆ serialize to blob ΓåÆ call Tauri to update the blob hash in the workspace index.

#### D4 ΓÇö Telemetry Footer (`components/Footer/TelemetryBar.tsx`)

```
[ ΓùÅ Connected ]  Peers: 3  |  Cache: 2.3 / 5.0 GB  |  Γåæ 1.2 MB/s  Γåô 4.5 MB/s
```

- Polls `get_storage_stats()` every 5 seconds via Tauri IPC.
- Uses Tauri event listener for real-time peer connection/disconnection events.

#### D5 ΓÇö Settings Panel (`components/Settings/CacheSlider.tsx`)

- Slider: 3 GB ΓåÆ 50 GB, with a "Full Mirror" toggle at the end.
- On change: call `set_cache_limit(bytes, full_mirror)` ΓåÆ immediately triggers eviction check.

---

## 5. Security Requirements

> These are **non-negotiable**. The security audit step (Step 17) must verify each point.

1. **Chunk integrity:** Every blob received from a remote peer passes BLAKE3 verification inside `iroh-blobs` before being written to disk. This is built-in to Iroh ΓÇö ensure it is not bypassed.
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

**`build.yml`** ΓÇö triggers on push to `main` and PRs:
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

**`test.yml`** ΓÇö triggers on all PRs:
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
| **M5: Anchor Online** | VPS anchor holds all blobs; offline student can retrieve files at Γëñ 150 KB/s. |
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

---

## 11. Phase 2 — Campus Hardening, UX & Autonomous P2P Resilience

> **For the executing agent:** This section defines the Phase 2 implementation tasks. Each step includes exact acceptance criteria, file paths to create/modify, concrete data models, and DO/DON'T rules. Steps within the same Milestone may be parallelized across subagents; steps in later Milestones depend on earlier ones completing first.

---

### Milestone 7: Campus Stealth & Network Adaptation

---

#### Step 33 — Campus Wi-Fi Detection & Eduroam Stealth Mode

**Assigned subagent:** `RustBackendAgent` (backend engine) + `FrontendAgent` (settings UI)

**Files to create/modify:**
- `src-tauri/src/p2p/network_profile.rs` — **NEW** — SSID detection + profile state machine
- `src-tauri/src/p2p/engine.rs` — add `reload_discovery_config(profile)` method
- `src-tauri/src/commands/peer_cmds.rs` — add `get_network_profile`, `set_network_profile` commands
- `src/components/Settings/NetworkSettings.tsx` — **NEW** — SSID rules table + manual override toggle
- `src/components/Footer/TelemetryBar.tsx` — add network profile badge

**Behaviour specification:**

1. **SSID Detection (Rust, background task, runs every 30 seconds):**
   - On **Windows**: call `netsh wlan show interfaces` via `std::process::Command` and parse the `SSID` field.
   - On **Linux**: call `iwgetid -r` via `std::process::Command`.
   - If SSID matches any entry in the `network_ssid_rules` SQLite table with `profile = 'campus'`, activate **Campus Stealth Profile**.
   - If SSID matches an entry with `profile = 'trusted'`, activate **Trusted/Home Profile**.
   - If SSID is **unrecognized** (not in the table), emit Tauri event `network-new-ssid` to the frontend so a toast can ask the user to classify it. Default to `'campus'` (conservative) until classified.
   - If SSID cannot be determined (VPN, Ethernet, detection failure), default to `'campus'` profile.

2. **Campus Stealth Profile — what changes:**
   - **DISABLE** BitTorrent DHT discovery (remove `discovery_dht()` from `Endpoint::builder()`).
   - **DISABLE** local mDNS / LAN Swarm discovery (remove `discovery_local_network()`).
   - **ENABLE** relay-only routing: `RelayMode::Custom(relay_url)` with relay on port 443.
   - **Background bandwidth cap**: enforce `MAX_CAMPUS_BACKGROUND_BPS` (default 512 KB/s) on all auto-fill and seeding tasks. User-initiated downloads (file clicks) remain uncapped.
   - **Lecture-hour throttle**: between 09:00 and 17:00 local time on weekdays, further cap background traffic to `MAX_LECTURE_BACKGROUND_BPS` (default 128 KB/s).
   - ❌ Do NOT stop the Iroh node. Only reconfigure discovery and routing layers.

3. **Trusted/Home Profile — what changes:**
   - Re-enable all three discovery layers: `discovery_local_network()`, `discovery_n0()`, `discovery_dht()`.
   - Remove background bandwidth cap (or restore user-configured cap).

4. **SQLite schema addition (new migration 0008):**
   ```sql
   CREATE TABLE IF NOT EXISTS network_ssid_rules (
       ssid     TEXT PRIMARY KEY,
       profile  TEXT NOT NULL CHECK(profile IN ('campus', 'trusted')),
       added_at INTEGER NOT NULL
   );
   -- Pre-seed known campus SSIDs
   INSERT OR IGNORE INTO network_ssid_rules(ssid, profile, added_at)
       VALUES ('eduroam', 'campus', unixepoch());
   INSERT OR IGNORE INTO network_ssid_rules(ssid, profile, added_at)
       VALUES ('campusroam', 'campus', unixepoch());
   ```

5. **LEDBAT-style adaptive congestion backoff:**
   - In the auto-fill background worker, measure round-trip latency to the active relay once per minute via a lightweight QUIC ping.
   - If measured RTT exceeds `LEDBAT_RTT_THRESHOLD_MS` (default 150 ms), reduce background upload rate by 50% until RTT recovers.

6. **New IPC commands:**
   ```rust
   #[tauri::command]
   async fn get_network_profile(state: State<'_, AppState>) -> Result<NetworkProfileInfo, String>;
   // Returns: { profile: "campus" | "trusted" | "unknown", ssid: Option<String>, lecture_throttle_active: bool }

   #[tauri::command]
   async fn set_network_profile_override(profile: String, state: State<'_, AppState>) -> Result<(), String>;
   // Manually force a profile; resets on next SSID change

   #[tauri::command]
   async fn add_ssid_rule(ssid: String, profile: String, state: State<'_, AppState>) -> Result<(), String>;

   #[tauri::command]
   async fn list_ssid_rules(state: State<'_, AppState>) -> Result<Vec<SsidRule>, String>;

   #[tauri::command]
   async fn delete_ssid_rule(ssid: String, state: State<'_, AppState>) -> Result<(), String>;
   ```

7. **Frontend — TelemetryBar badge:**
   - Show `🛡 Campus Mode [eduroam]` in amber when Campus Stealth is active.
   - Show `🏠 Home Network` in green when Trusted profile is active.
   - 1-click on the badge opens the Network Settings panel.

8. **Frontend — NetworkSettings.tsx:**
   - Editable SSID rules table: columns `SSID | Profile | Actions (Delete)`.
   - Manual override toggle: `Force Campus Mode` / `Force Trusted Mode` / `Auto-Detect`.
   - Toast notification for new unrecognized SSIDs (calls `add_ssid_rule` on user answer; wording: "Is this a personal or public/campus network?").

**Rules:**
- ❌ Do NOT make any HTTP calls to detect network type — use OS CLI tools only.
- ❌ Do NOT use JavaScript `navigator.connection` — only use the Tauri IPC `get_network_profile` command.
- ✅ Detection must work when the app is running in the background (tray mode).

**Acceptance criteria:**
- [ ] Unit test: when SSID = `eduroam`, returned config has DHT and mDNS disabled.
- [ ] `get_network_profile` returns correct profile within 35 seconds of SSID change.
- [ ] TelemetryBar shows correct campus/home badge.
- [ ] SSID rules table persists entries across app restarts.
- [ ] Unit test: lecture-hour throttle activates at 09:00 and deactivates at 17:00 (use mocked time).

---

#### Step 34 — First-Run Onboarding Wizard & Connection Doctor

**Assigned subagent:** `FrontendAgent` + `RustBackendAgent` (IPC for diagnostics)

**Files to create/modify:**
- `src/components/Onboarding/OnboardingWizard.tsx` — **NEW** — multi-step wizard shell
- `src/components/Onboarding/OnboardingSimple.tsx` — **NEW** — simplified path
- `src/components/Onboarding/OnboardingAdvanced.tsx` — **NEW** — advanced path
- `src/components/Onboarding/CredentialsStep.tsx` — **NEW** — passphrase input (component boundary for future OTP swap)
- `src/components/Onboarding/ConnectionDoctor.tsx` — **NEW** — in-app diagnostic tool
- `src-tauri/src/commands/diagnostic_cmds.rs` — **NEW** — backend diagnostic tests
- `src-tauri/src/storage/db.rs` — add `onboarding_complete` setting key

**First-run detection:**
- On app start, check: `SELECT value FROM settings WHERE key = 'onboarding_complete'`.
- If the row does not exist or value is `'false'`, show `OnboardingWizard` fullscreen before the main UI.
- After wizard completes, write: `INSERT OR REPLACE INTO settings VALUES ('onboarding_complete', 'true')`.

**Wizard step flow:**
```
Step 0: Mode Select
  → [Quick Setup]    → OnboardingSimple  (S1 → S2 → Finish)
  → [Advanced Setup] → OnboardingAdvanced (A1 → A2 → A3 → A4 → A5 → Finish)

OnboardingSimple:
  S1: Join Swarm
      - Text input: "Study Group Passphrase"
      - Explanation: "This shared secret connects you to your classmates' devices."
      - Writes to settings: 'network_secret'
  S2: Storage Quota
      - Slider: 3 GB → Full Mirror (default: 5 GB)
      - Explanation: "Disk space you donate to store and share course files."
      - Writes to settings: 'cache_limit_bytes'

OnboardingAdvanced:
  A1: Join Swarm (same as S1; leave blank = join public swarm)
  A2: Storage Quota (same as S2)
  A3: Network Profile
      - "Are you on a university Wi-Fi (Eduroam/campusroam)?"
      - Toggle: "Yes — enable campus stealth mode by default"
      - Toggle: "Warn me when seeding on mobile hotspots (metered connections)"
  A4: Bandwidth Caps
      - "Background seeding speed cap" slider: Off / 128 KB/s / 512 KB/s / 2 MB/s / Unlimited
      - Toggle: "Pause background seeding when on battery"
  A5: Relay Server
      - Text input: relay URL (default shown: https://use1-1.relay.iroh.network/)
      - Explanation: "For private class networks, your group admin provides this URL."
```

**Connection Doctor (accessible from Settings → Network → 'Run Diagnostics'):**

```rust
// New IPC command in diagnostic_cmds.rs:
#[tauri::command]
async fn run_connection_doctor(state: State<'_, AppState>, app: AppHandle) -> Result<(), String>;
// Does NOT return a result directly — emits Tauri events as each test completes.
// Events: 'doctor-test-result' { test: String, ok: bool, detail: String }
// Tests run in this order: "mdns", "quic_udp", "relay_tcp", "nat_type"
// Final event: 'doctor-complete' { advice: Vec<String> }
```

**Advice strings the backend MUST generate:**
- If `!quic_udp_ok && relay_tcp_ok`: `"Eduroam is blocking direct UDP. All traffic routes through the relay — transfers will be slightly slower, but everything works."`
- If `!relay_tcp_ok`: `"Cannot reach the relay server. Check your internet connection, or ask your group admin for the correct relay URL."`
- If `nat_type == "symmetric"`: `"Your network uses symmetric NAT. The relay is required for all connections — this is normal on Eduroam."`
- If `mdns_ok`: `"Found campus peers on this Wi-Fi — direct LAN connections are available and fast."`

**Frontend — ConnectionDoctor.tsx:**
- Shows each test as a row: `🟡 Testing...` → `✅ Pass` or `❌ Fail` as events arrive.
- Renders `advice` strings in an amber info box at the bottom.
- 'Run Again' button re-triggers the command.

**Rules:**
- ❌ Do NOT block the Tauri main thread during diagnostics — run all tests via `tokio::spawn` and emit results as Tauri events.
- ❌ Do NOT close the wizard without writing `onboarding_complete = 'true'` to SQLite (risk: wizard reappears every launch).
- ✅ The passphrase screen MUST use a `<CredentialsStep onComplete={(passphrase) => ...} />` component boundary so it can be replaced with an email OTP flow in a future step without rewriting the wizard.

**Acceptance criteria:**
- [ ] Wizard appears on first launch (no `onboarding_complete` key in SQLite).
- [ ] Wizard does NOT appear on subsequent launches after completion.
- [ ] All wizard settings (passphrase, quota, relay URL, campus mode toggle) are written to SQLite and readable on next launch.
- [ ] Connection Doctor emits events for each test; frontend transitions from 🟡 to ✅/❌ in real time.
- [ ] At least one advice string is returned when the relay is unreachable.

---

### Milestone 8: Smart Scarcity Caching & Hardware Protection

---

#### Step 35 — Smart Scarcity Replication & SSD Wear Defense

**Assigned subagent:** `RustBackendAgent`

**Files to create/modify:**
- `src-tauri/src/storage/eviction.rs` — extend eviction algorithm (seed-count-first order, dwell guard)
- `src-tauri/src/storage/scarcity.rs` — **NEW** — scarcity replication background worker
- `src-tauri/src/storage/db.rs` — add migrations 0009

**New SQLite migration 0009:**
```sql
-- SSD wear defense: track when blobs were first written to disk
ALTER TABLE local_blobs ADD COLUMN written_at INTEGER;
-- Backfill: UPDATE local_blobs SET written_at = last_accessed_at WHERE written_at IS NULL;

-- Transfer resume queue (also used by Step 39)
CREATE TABLE IF NOT EXISTS transfer_resume_queue (
    hash           TEXT PRIMARY KEY,
    name           TEXT,
    total_bytes    INTEGER NOT NULL,
    received_bytes INTEGER NOT NULL DEFAULT 0,
    source_peer    TEXT,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

-- Daily SSD write budget tracker
CREATE TABLE IF NOT EXISTS write_budget (
    date_key      TEXT PRIMARY KEY,  -- 'YYYY-MM-DD'
    bytes_written INTEGER NOT NULL DEFAULT 0
);
```

**Eviction algorithm changes (eviction.rs) — three new hard rules:**

1. **Dwell time guard:** Before evicting any blob, check `written_at`. If `now - written_at < 86400` seconds (24 hours), **skip** that blob entirely and move to the next candidate. Never evict a blob written within the last 24 hours.
2. **Seed-count-first eviction order:** Sort eviction candidates by `(remote_seed_count DESC, last_accessed_at ASC)`. This means blobs that many other peers already hold are evicted before rare ones. Breaking ties by LRU (oldest accessed first).
3. **Sole-seed hard block (from Step 28, still applies):** If `remote_seed_count = 0`, never evict regardless of dwell time or age.

**Scarcity replication worker (scarcity.rs) — exact algorithm:**

```
Worker loop interval: SCARCITY_CHECK_INTERVAL_SECS (default 600 = 10 minutes, configurable via settings)

Each iteration:
  1. Find the most endangered blob not locally stored:
     - A blob is "endangered" if it appears in the workspace catalog but NOT in the local iroh-blobs store,
       AND the count of online peers seeding it (last_seen_at > now - 300 seconds) is exactly 1.
     - Query: blobs with the fewest active seeds first, smallest file size first as tiebreaker.
     - LIMIT 1 per iteration — only download one file per interval to pace SSD writes.

  2. If nothing endangered, sleep until next interval.

  3. Daily write budget check:
     - Read today's bytes_written from write_budget table.
     - If bytes_written + file_size > DAILY_WRITE_BUDGET_BYTES (default 3 GB), skip and sleep.
     - User-initiated downloads do NOT count toward this budget.

  4. Network profile check:
     - If current profile is 'campus' AND current local time is between 09:00 and 17:00 on a weekday,
       skip auto-fill entirely this iteration (do not auto-replicate during lecture hours on campus).

  5. Peer availability check:
     - If no peer currently online has the blob, skip (cannot download from nobody).

  6. Begin downloading the blob from any available peer seed using existing blob download IPC.
     On completion: add file_size to today's write_budget entry.

  7. After full download: verify BLAKE3 hash. If mismatch, delete the downloaded data and log error.

  8. Emit Tauri event 'scarcity-replicated' { hash, name, size_bytes } so the UI can show a toast.
```

**In-memory chunk buffering (apply to ALL blob downloads, not just scarcity worker):**
- Use a `Vec<u8>` staging buffer or `tokio::io::BufWriter` to accumulate chunks in memory.
- After all chunks received, verify BLAKE3 root hash matches the expected hash.
- Only then call `store.import_bytes()` to write to disk.
- If hash mismatch: discard buffer, log warning, do NOT write corrupt data.
- ❌ Do NOT write chunks one-by-one to disk in a hot loop.

**Rules:**
- ❌ Do NOT evict blobs written within the last 24 hours (dwell guard).
- ❌ Do NOT auto-replicate on campus network during 09:00–17:00 weekdays.
- ❌ Do NOT exceed daily write budget for auto-fill operations.
- ✅ Always verify BLAKE3 hash before committing to the blob store.
- ✅ User-initiated downloads bypass ALL throttles and budgets.

**Acceptance criteria:**
- [ ] Unit test: 3 blobs all with `written_at = now`, quota exceeded → eviction skips ALL three (dwell guard).
- [ ] Unit test: blob with `remote_seed_count = 5` evicted before blob with `remote_seed_count = 1`, even if the former was accessed more recently.
- [ ] Unit test: blob with `remote_seed_count = 0` never evicted regardless of size or age.
- [ ] Unit test: daily write budget = 100 MB; two 60 MB auto-fills → second blocked; explicit user download still succeeds.
- [ ] Scarcity worker downloads a 1-seed blob within 2 worker intervals of it appearing in the catalog.

---

#### Step 36 — Sole Seed Presence & Desktop Alerts

**Assigned subagent:** `FrontendAgent` + `RustBackendAgent`

**Files to create/modify:**
- `src-tauri/src/commands/peer_cmds.rs` — add `get_sole_seed_files` command
- `src/components/SoleSeedBanner.tsx` — **NEW** — floating dismissible banner
- `src-tauri/src/main.rs` — add taskbar/tray flash on window close attempt

**Backend IPC:**
```rust
#[tauri::command]
async fn get_sole_seed_files(state: State<'_, AppState>) -> Result<Vec<SoleSeedFile>, String>;
// SoleSeedFile { hash: String, name: Option<String>, size_bytes: u64 }
//
// A file qualifies as "sole seed" when:
//   - This node stores it locally (hash is in the iroh-blobs store), AND
//   - No other peer has been seen seeding it in the last 5 minutes:
//     SELECT COUNT(*) FROM file_seeds WHERE hash = ? AND peer_node_id != <our_node_id>
//       AND last_seen_at > unixepoch() - 300  →  = 0
```

**Frontend — SoleSeedBanner.tsx:**
- Poll `get_sole_seed_files` every 60 seconds.
- If result is non-empty AND not dismissed this session: render a dismissible banner at the top of the main content area (between header and file explorer):
  ```
  ⚡ You are the only online copy of 2 course files. Keep the app running to protect them.
  [Keep Seeding]  [Show Files]  [×]
  ```
  - `Show Files`: navigates to file explorer and applies a temporary "sole seed" filter.
  - `×`: dismisses for the current session only. Re-appears on next launch if still sole seed.
- If result is empty, hide the banner.

**Quit dialog on window close (in main.rs `CloseRequested` handler):**
- If `get_sole_seed_files` returns a non-empty list when user requests quit:
  - Show a native OS dialog (NOT an HTML modal — use `tauri::api::dialog::blocking::MessageDialogBuilder`):
    ```
    "You are the only copy of 2 rare course files.
     If you quit, these files may become unavailable to your classmates.
     [Keep Seeding in Tray]  [Quit Anyway]"
    ```
  - `Keep Seeding in Tray`: hide window to tray as normal, do NOT quit.
  - `Quit Anyway`: proceed with shutdown.
- If `get_sole_seed_files` returns empty: proceed with normal close-to-tray behaviour. Do NOT show the dialog.

**Taskbar flash (Windows only):**
- Before showing the quit dialog: call `window.set_progress_bar(ProgressBarState::Indeterminate)` to flash the taskbar button.
- After dialog resolves: call `window.set_progress_bar(ProgressBarState::None)` to restore it.

**Tray icon state:**
- When sole seed files exist: switch tray icon to the amber warning variant (`tray-warning.png`).
- When no sole seed files: restore default tray icon.
- Poll this check every 60 seconds alongside the banner poll.

**Rules:**
- ❌ Do NOT block the UI thread while checking sole seed status — use async IPC.
- ❌ Do NOT show the quit dialog when minimizing or hiding to tray without user requesting close.
- ❌ Do NOT dismiss the banner permanently — session-only dismissal only.

**Acceptance criteria:**
- [ ] Unit test: `get_sole_seed_files` returns correctly populated list with mocked seed table.
- [ ] Banner appears when sole seed files exist and disappears when seed count rises above 0.
- [ ] `×` button suppresses banner for the session; re-appears on next launch if still sole seed.
- [ ] Quit dialog appears on `CloseRequested` when sole seeds exist; does NOT appear when count is 0.
- [ ] Tray icon changes to amber variant when sole seed files exist.

---

### Milestone 9: Data Integrity, Moderation & Resilient UX

---

#### Step 37 — Strict Path Immutability & File Collision Resolution

**Assigned subagent:** `RustBackendAgent` + `FrontendAgent`

**Files to modify:**
- `src-tauri/src/storage/blob_tracker.rs` — enforce path lock on `upsert_blob`
- `src-tauri/src/p2p/sync.rs` — enforce lock on incoming `FileAdded` and `FileMoved` messages
- `src-tauri/src/storage/path_utils.rs` — add Windows-safe normalization function
- `src-tauri/src/commands/file_cmds.rs` — add executable warning before download
- `src/components/Explorer/FileRow.tsx` — render executable warning modal

**Strict immutability (enforced in Rust, NOT just the UI):**

Rule 1 — Reject path conflicts in `upsert_blob`:
```rust
// Before inserting a new blob record, check if the virtual path is already taken by a DIFFERENT hash:
// SELECT hash FROM local_blobs WHERE name = ? LIMIT 1
// If found and hash != new_hash → return Err("Path already occupied by a different file. Delete the existing file first.")
// If found and hash == new_hash → no-op (same file uploaded twice), return Ok(())
```

Rule 2 — Same check applies in sync.rs `FileAdded` handler:
- When receiving `SyncMessage::FileAdded` from a peer, run the same conflict check.
- If the path is occupied by a different hash: do NOT overwrite. Run the conflict resolution algorithm below instead.

**Offline split-brain conflict resolution algorithm (in sync.rs):**
```
When an incoming FileAdded has a path that locally holds a different hash:

  Compare created_at timestamps (Unix seconds stored in BlobRecord):
  - If incoming.created_at < local.created_at:
      (incoming file is older → it wins the original path)
      Rename local record: new name = strip_ext(original) + " (" + our_short_node_id + ")" + ext
      Accept incoming record at the original path.
      Broadcast SyncMessage::FileMoved for the renamed local record.
  - If incoming.created_at > local.created_at:
      (local file is older → it wins)
      Rename incoming: new name = strip_ext(original) + " (" + sender_short_node_id + ")" + ext
      Insert incoming record under the renamed path.
      Broadcast SyncMessage::FileMoved for the renamed incoming record.
  - If incoming.created_at == local.created_at:
      Use lexicographic comparison of BLAKE3 hashes. The lexicographically smaller hash wins
      the original path. The larger hash is renamed.

  short_node_id = first 8 hex characters of the NodeId.
  Conflict resolution must be deterministic: if the same two nodes reconnect multiple times,
  they must always produce the same winner without oscillating.
```

**Cross-platform path normalization (`normalize_virtual_path` in path_utils.rs):**
```rust
/// Normalize and validate a virtual file path component for cross-platform safety.
/// Returns Err if any rule is violated; returns Ok(normalized) on success.
///
/// Rules applied in order:
/// 1. Apply NFC Unicode normalization.
/// 2. Reject strings containing null bytes (\0).
/// 3. Reject strings containing Windows-illegal characters: < > : " / \ | ? *
/// 4. Reject strings whose last path component (after final '/') matches a Windows reserved
///    device name (case-insensitive): CON, PRN, AUX, NUL, COM0-COM9, LPT0-LPT9.
/// 5. Reject paths exceeding 200 characters total.
/// 6. Strip leading and trailing whitespace and dots from each path component.
/// 7. After normalization, perform a case-insensitive uniqueness check against existing
///    entries in the same virtual folder. Return Err if a case-insensitive match is found
///    with a different hash (prevents "notes.md" and "Notes.md" coexisting).
pub fn normalize_virtual_path(input: &str, folder_sibling_names: &[&str]) -> Result<String, String>
```

**Executable file download warning:**
- Dangerous extensions list: `.exe`, `.msi`, `.bat`, `.cmd`, `.ps1`, `.sh`, `.elf`, `.bin`, `.dll`, `.so`, `.dylib`, `.vbs`
- When user clicks "Download" or "Open" on a file with one of these extensions, show a modal BEFORE initiating the download:
  ```
  ⚠️ Executable File Warning
  "setup.exe" is a program file, not a document.
  Running files from untrusted sources can harm your computer.
  The app will save this file to your Downloads folder.
  It will NOT be opened or run automatically.
  [Save to Downloads]  [Cancel]
  ```
- After the user confirms: save the file to `dirs::download_dir()` using `std::fs::copy` from the blob store. Do NOT use `open`/`xdg-open`/`ShellExecute` under any circumstances.
- ❌ The app MUST NEVER execute or open executable files via any OS handler.

**Rules:**
- ❌ Do NOT silently overwrite a path that holds a different hash.
- ❌ Do NOT allow opening any executable file type from within the app.
- ✅ Conflict resolution must be deterministic — same winner every time for the same two blobs.

**Acceptance criteria:**
- [ ] Unit test: `upsert_blob` with duplicate path and different hash → returns `Err`.
- [ ] Unit test: `upsert_blob` with duplicate path and identical hash → returns `Ok(())`, no duplicate row.
- [ ] Unit test: offline conflict — node A's older `created_at` file wins; node B's file renamed with B's short node ID.
- [ ] Unit test: `normalize_virtual_path("CON.pdf", &[])` returns `Err`.
- [ ] Unit test: `normalize_virtual_path("notes<1>.md", &[])` returns `Err`.
- [ ] Unit test: when `"Notes.md"` already exists, `normalize_virtual_path("notes.md", &["Notes.md"])` returns `Err` (case collision).
- [ ] Executable download modal appears for `.exe`, `.bat`, `.sh`, `.ps1`; file is NOT opened automatically.

---

#### Step 38 — Study Ratings, Empty Folder Rules & Emergency Quarantine

**Assigned subagent:** `RustBackendAgent` + `FrontendAgent`

**Files to create/modify:**
- `src-tauri/src/storage/db.rs` — add migration 0010
- `src-tauri/src/storage/blob_tracker.rs` — add rating CRUD and quarantine logic
- `src-tauri/src/p2p/sync.rs` — add `FileRated`, `FileQuarantineReport` message variants
- `src-tauri/src/commands/file_cmds.rs` — add IPC commands for rating, starring, quarantine, folder delete
- `src/components/Explorer/FileRow.tsx` — add star, upvote/downvote UI elements; quarantine red styling

**New SQLite migration 0010:**
```sql
CREATE TABLE IF NOT EXISTS file_ratings (
    hash          TEXT NOT NULL,
    voter_node_id TEXT NOT NULL,
    rating        INTEGER NOT NULL CHECK(rating IN (-1, 0, 1)),  -- -1=down, 0=neutral, 1=up
    voted_at      INTEGER NOT NULL,
    PRIMARY KEY (hash, voter_node_id)
);
CREATE INDEX IF NOT EXISTS idx_ratings_hash ON file_ratings(hash);

CREATE TABLE IF NOT EXISTS file_stars (
    hash          TEXT PRIMARY KEY,  -- local only, NOT synced to peers
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
ALTER TABLE local_blobs ADD COLUMN quarantine_expires_at INTEGER;  -- NULL = not quarantined
```

**Upvotes / downvotes:**
- Each node stores one vote per file in `file_ratings` (overwrite on re-vote).
- Net score = `SUM(rating)` across all voters.
- UI displays: `▲ 12 ▼ 3` or simplified net `+9`.
- Synced via new message variant: `SyncMessage::FileRated { hash: String, voter_node_id: String, rating: i8 }`.
- **Seeding priority effect:** In the scarcity worker replication queue, multiply seed scarcity by quality factor. Files with net rating ≤ −3 are deprioritized (placed last in the queue). They are still seeded if already in cache — just not eagerly replicated.

**Star bookmarks:**
- Stored in `file_stars` table. Local only — NOT synced to peers.
- Starred files display a ★ icon in the file explorer.
- A "Starred" toggle in the file explorer header filters to starred files only.

**Emergency quarantine — exact trigger and behaviour:**
```
Quarantine trigger (run this check after every INSERT into quarantine_reports):
  SELECT COUNT(*) FROM quarantine_reports
  WHERE hash = ? AND reported_at > unixepoch() - 3600  →  >= 5?

If triggered:
  SET local_blobs.is_quarantined = 1
  SET local_blobs.quarantine_expires_at = unixepoch() + 604800  (7 days)
  DO NOT delete blob data from disk or iroh-blobs store yet.
  Emit Tauri frontend event 'file-quarantined' { hash, name } to update the UI.

Quarantine behaviour:
  - Quarantined files are NOT served to peers (do not seed them — skip in `get_sole_seed_files` too).
  - Quarantined files are NOT shown in the file explorer by default.
  - Settings → Moderation: "Show Quarantined Files" toggle reveals them with ⛔ red border.
  - Wherever shown, quarantined files display: "⛔ DANGEROUS — Reported by community"

Quarantine reversal threshold:
  - Reversal requires >= 80% of all nodes who have EITHER rated OR reported this file to cast a
    "clear" vote. A clear vote = rating of +1 on a currently quarantined file.
  - Formula: clear_votes / (clear_votes + report_votes) >= 0.80
  - If threshold met: SET is_quarantined = 0, SET quarantine_expires_at = NULL.

Automatic quarantine expiry:
  - After 7 days (quarantine_expires_at < unixepoch()): SET is_quarantined = 0 automatically.
  - The blob is NOT deleted. It returns to normal catalog visibility.

Anchor admin hook (placeholder for future step):
  - Reserve a code path for SyncMessage::AdminAction { action: "ClearQuarantine" | "ForceDelete", hash }
    signed by the anchor node's public key. Add a TODO(anchor-admin) comment at this hook point.
    DO NOT implement anchor admin now.
```

**Empty folder deletion rules:**
- Folders in the virtual tree are purely virtual — derived from `name` path prefixes in `local_blobs`.
- Folders have NO owner record. Any student can create or delete a folder.
- Before deleting a folder, check: `SELECT COUNT(*) FROM local_blobs WHERE name LIKE 'FolderPath/%'`.
- If > 0, return `Err("Cannot delete a folder that still contains files. Remove all files first.")`.
- ❌ There is no concept of folder ownership. No student "owns" a folder.

**New IPC commands:**
```rust
#[tauri::command]
async fn rate_file(hash: String, rating: i8, state: State<'_, AppState>) -> Result<(), String>;
// rating must be -1, 0, or 1. Writes to file_ratings and broadcasts SyncMessage::FileRated.

#[tauri::command]
async fn toggle_star(hash: String, state: State<'_, AppState>) -> Result<bool, String>;
// Toggles file_stars entry. Returns new starred state. Local only, NOT synced.

#[tauri::command]
async fn report_dangerous(hash: String, state: State<'_, AppState>) -> Result<QuarantineStatus, String>;
// Adds to quarantine_reports. Checks if 5-report/1h threshold is met. Triggers quarantine if so.
// QuarantineStatus { is_quarantined: bool, report_count: u32 }

#[tauri::command]
async fn delete_folder(virtual_path: String, state: State<'_, AppState>) -> Result<(), String>;
// Returns Err if the folder contains any files (non-empty).
```

**Acceptance criteria:**
- [ ] Unit test: 5 quarantine reports within 1 hour → `is_quarantined = 1` written to DB.
- [ ] Unit test: 4 quarantine reports within 1 hour → `is_quarantined` remains 0.
- [ ] Unit test: quarantined file not returned in normal `list_all_blobs` query (filtered out by default).
- [ ] Unit test: `delete_folder` returns `Err` when folder has files; returns `Ok(())` when empty.
- [ ] Unit test: file with net rating = −4 placed after all files with net rating ≥ −3 in scarcity queue.
- [ ] `toggle_star` state persists across app restarts; file appears with ★ icon in explorer.

---

#### Step 39 — Persistent Transfer Resumption & Enhanced Offline Mode

**Assigned subagent:** `RustBackendAgent` + `FrontendAgent`

**Files to create/modify:**
- `src-tauri/src/storage/blob_tracker.rs` — add resume queue CRUD (table created in Step 35 migration 0009)
- `src-tauri/src/commands/file_cmds.rs` — integrate resume queue into download start/completion flow
- `src/components/OfflineBanner.tsx` — **NEW** — top-of-screen offline status bar
- `src/components/Explorer/FileTree.tsx` — add "Available Offline" filter toggle + cloud-dimmed rows
- `src/hooks/useConnectionStatus.ts` — **NEW** — Tauri event listener for connection state

**Transfer resume queue — exact integration into download flow:**
```
On download start (ANY download, user-initiated or scarcity auto-fill):
  INSERT OR REPLACE INTO transfer_resume_queue
  (hash, name, total_bytes, received_bytes=0, source_peer, created_at=now, updated_at=now)

During download — update progress periodically:
  UPDATE transfer_resume_queue SET received_bytes = ?, updated_at = now WHERE hash = ?
  Update every 10% progress OR every 5 seconds, whichever comes first.
  DO NOT update on every single chunk — this would cause excessive SSD writes.

On download completion (after BLAKE3 verification passes):
  DELETE FROM transfer_resume_queue WHERE hash = ?

On app startup — resume interrupted downloads:
  SELECT * FROM transfer_resume_queue ORDER BY created_at ASC
  For each row:
    - If a peer online has the blob: begin download, seeking to received_bytes offset.
    - If no peer online: leave the row in the table; retry on next swarm sync cycle (5s).

On app shutdown:
  DO NOT delete or modify transfer_resume_queue rows.
  DO NOT delete partial blob data from the iroh-blobs store.
  Partial data is preserved; full BLAKE3 verification happens on next resumption.
```

**Rules:**
- ❌ Do NOT delete `transfer_resume_queue` rows on clean shutdown.
- ❌ Do NOT restart a download from byte 0 if `received_bytes > 0`.
- ✅ After resumption completes, verify BLAKE3 root hash before marking as done.

**Offline Mode (frontend):**

Backend emits Tauri event `connection-status-changed` with payload `{ online: bool, peer_count: u32 }` whenever:
- Peer count transitions from 0 → >0 (came online).
- Peer count transitions from >0 → 0 (went offline).

`useConnectionStatus.ts` hook:
- Listens for `connection-status-changed` events.
- Exposes `{ isOnline: boolean, peerCount: number }` to consumers.
- Initial state from `get_storage_stats()` on mount.

**OfflineBanner.tsx:**
- Renders at top of main content area when `isOnline === false`:
  ```
  📡 Offline — viewing cached files only  [Retry Connection]
  ```
- `[Retry Connection]` calls new IPC `reconnect_swarm()` which forces the background loop to run immediately.
- Banner disappears when `isOnline` becomes true.
- ❌ Do NOT show when `peer_count = 0` but the app is still searching (distinguish "searching" from "offline"). "Offline" means the QUIC endpoint reports no reachable relay AND no local peers.

**Available Offline Filter (FileTree.tsx):**
- Add a toggle in the file explorer header: `☁ Show All` ↔ `💾 Offline Only`.
- When "Offline Only" active:
  - Show ONLY blobs where local node has the full data (`is_pinned = 1` OR hash exists in local iroh-blobs store).
  - Other rows: dimmed to 50% opacity with `☁` cloud icon in the status column.
  - Clicking a dimmed row while offline shows: `"This file is only available on peer devices. Connect to Wi-Fi to download it."`
- When filter is off (default): show all files.

**New IPC commands:**
```rust
#[tauri::command]
async fn reconnect_swarm(state: State<'_, AppState>) -> Result<(), String>;
// Triggers the background swarm loop to run immediately, skipping the 5-second wait.

#[tauri::command]
async fn get_transfer_queue(state: State<'_, AppState>) -> Result<Vec<TransferQueueItem>, String>;
// Returns all rows from transfer_resume_queue for display.
```

**Acceptance criteria:**
- [ ] Start downloading a 100 MB file, kill the app process, relaunch — download resumes from last checkpoint (not from byte 0).
- [ ] `OfflineBanner` appears when peer count drops to 0 (confirmed offline) and disappears when peer reconnects.
- [ ] "Available Offline" filter shows only locally cached files; cloud-only files are dimmed.
- [ ] Clicking a dimmed row while offline shows the correct message (not an error or crash).
- [ ] `transfer_resume_queue` rows persist in SQLite after a clean app shutdown.

---

#### Step 40 — Workspace Backup, Folder ZIP Export & Non-Intrusive Updates

**Assigned subagent:** `FrontendAgent` + `RustBackendAgent`

**Files to create/modify:**
- `src-tauri/src/commands/backup_cmds.rs` — **NEW**
- `src/components/Settings/BackupSettings.tsx` — **NEW**
- `src-tauri/src/main.rs` — hook auto-updater into startup and shutdown events

**Full App Data Backup:**
```rust
#[tauri::command]
async fn export_app_backup(app: AppHandle, state: State<'_, AppState>) -> Result<(), String>;
// 1. Show a native save-file dialog for the destination .zip path.
// 2. Create a ZIP archive at that path containing ONLY:
//      iroh_secret.key   — the node's Ed25519 identity key
//      p2pstud.db        — the full SQLite database
// 3. Do NOT include blob data (files are too large; they will be re-downloaded from peers on restore).
// 4. On completion, show a native info dialog: "Backup saved. Keep this file secure —
//    it contains your identity key."
```

**Folder ZIP Export:**
```rust
#[tauri::command]
async fn export_folder_zip_preflight(
    virtual_path: String,
    state: State<'_, AppState>
) -> Result<ExportPreflightInfo, String>;
// ExportPreflightInfo { locally_available: u32, needs_download: u32, total_size_bytes: u64 }
// Does NOT start any downloads. Returns info so the frontend can ask the user to confirm.

#[tauri::command]
async fn export_folder_zip(
    virtual_path: String,
    dest_path: String,
    state: State<'_, AppState>,
    app: AppHandle
) -> Result<(), String>;
// 1. Download missing blobs from peers (adds them to transfer_resume_queue).
// 2. When all blobs are locally available, write them to ZIP at dest_path.
//    Preserve virtual path structure inside ZIP (e.g., /CS101/Week1/Lecture.pdf).
// 3. Emit progress events 'export-progress' { done: u32, total: u32 } during download phase.
```

**Frontend flow for folder export:**
```
User right-clicks a folder → "Export as ZIP"
  → call export_folder_zip_preflight(virtual_path)
  → If needs_download > 0: show modal:
      "2 of 7 files (1.2 GB) need to be downloaded from peers before exporting.
       This may take a while depending on peer availability.
       [Download & Export]  [Export Cached Only]  [Cancel]"
  → If user chooses "Export Cached Only": call export_folder_zip with only locally available blobs.
  → If user chooses "Download & Export": call export_folder_zip (downloads then exports all).
  → Show save-file dialog for ZIP destination.
```

**Auto-Updater integration:**
- Use `tauri-plugin-updater` (built into Tauri v2).
- Configure `updater.endpoints` in `tauri.conf.json` to point to GitHub releases JSON.

**On app startup (10 seconds after main window loads):**
```
Spawn tokio::spawn:
  sleep 10 seconds
  Call updater.check()
  If update available: emit Tauri event 'update-available' { version, notes, download_size_bytes }
  Frontend shows a non-blocking toast (NOT a modal, NOT a blocking dialog):
    "🔄 Update available: v1.x.x  [Update on Next Restart]  [Later]"
  If user clicks "Update on Next Restart": call updater.download() in background.
    After download completes: set a flag in SQLite: 'update_pending' = 'true'
  If user clicks "Later": dismiss toast. Do not ask again this session.
```

**On app shutdown (before the process exits):**
```
In CloseRequested handler, AFTER the sole-seed check (Step 36):
  If settings 'update_pending' == 'true':
    Show native dialog:
      "An update is ready to install. Install now?"
      [Install & Restart]  [Quit Without Installing]
    If "Install & Restart": call updater.install() (applies update and relaunches).
    If "Quit Without Installing": proceed with normal quit.
```

**Rules:**
- ❌ Do NOT apply updates silently without user confirmation.
- ❌ Do NOT show the update prompt while the user is actively reading a document or during an in-progress transfer.
- ✅ The startup check must be delayed by exactly 10 seconds so the app feels fast on launch.
- ✅ The startup notification must be a toast (non-blocking), NOT a dialog that interrupts the user.

**Acceptance criteria:**
- [ ] `export_app_backup` creates a valid ZIP at the chosen path containing exactly `iroh_secret.key` and `p2pstud.db`.
- [ ] `export_folder_zip_preflight` returns correct `needs_download` count before writing any data.
- [ ] Update toast appears on startup (after 10s) when a newer version is available; NOT when no update exists.
- [ ] Update install dialog appears on shutdown when update is downloaded (`update_pending = 'true'`).
- [ ] No update-related dialog appears on startup.

---

#### Step 41 — Bilateral Seeding Receipts & Catalog Lifecycle Pruning

**Assigned subagent:** `RustBackendAgent`

**Files to create/modify:**
- `src-tauri/src/storage/db.rs` — add migrations 0011 and 0012
- `src-tauri/src/storage/blob_tracker.rs` — add receipt CRUD
- `src-tauri/src/p2p/sync.rs` — add `SeedingReceipt` message variant and verification
- `src-tauri/src/storage/pruning.rs` — **NEW** — lifecycle pruning background worker

**New SQLite migrations:**
```sql
-- Migration 0011 — seeding receipts for future Sybil-resistant governance
CREATE TABLE IF NOT EXISTS seeding_receipts (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    hash              TEXT NOT NULL,
    seeder_node_id    TEXT NOT NULL,
    recipient_node_id TEXT NOT NULL,
    bytes_transferred INTEGER NOT NULL,
    issued_at         INTEGER NOT NULL,
    signature         TEXT NOT NULL
    -- Signing payload (UTF-8 string):
    -- "receipt:{hash}:{seeder_node_id}:{recipient_node_id}:{bytes_transferred}:{issued_at}"
    -- Signed by the RECIPIENT using their Ed25519 SecretKey.
    -- Verified using the RECIPIENT's public key (their NodeId).
);
CREATE INDEX IF NOT EXISTS idx_receipts_seeder ON seeding_receipts(seeder_node_id, issued_at);
CREATE INDEX IF NOT EXISTS idx_receipts_hash   ON seeding_receipts(hash);

-- Migration 0012 — lifecycle aging metadata
ALTER TABLE local_blobs ADD COLUMN catalog_first_seen_at  INTEGER;  -- Unix timestamp
ALTER TABLE local_blobs ADD COLUMN last_any_seed_seen_at  INTEGER;  -- Unix timestamp
-- Backfill: UPDATE local_blobs SET catalog_first_seen_at = last_accessed_at WHERE catalog_first_seen_at IS NULL;
```

**Seeding receipt issuance (runs after every completed download):**
```
After full BLAKE3 verification passes on a downloaded blob:
  seeder_node_id = the peer we downloaded from
  recipient_node_id = our own NodeId
  payload = format!("receipt:{hash}:{seeder_node_id}:{recipient_node_id}:{bytes}:{now_unix}")
  signature = our_secret_key.sign(payload.as_bytes())  // Ed25519 sign

Store locally:
  INSERT INTO seeding_receipts (hash, seeder_node_id, recipient_node_id, bytes_transferred, issued_at, signature)

Send to seeder peer:
  SyncMessage::SeedingReceipt { hash, seeder_node_id, recipient_node_id, bytes_transferred, issued_at, signature_hex }
```

**Receipt verification (when receiving a SeedingReceipt over QUIC):**
```
1. Reconstruct payload string: "receipt:{hash}:{seeder_node_id}:{recipient_node_id}:{bytes}:{issued_at}"
2. Parse recipient_node_id as an iroh::NodeId (which contains the Ed25519 public key).
3. Verify Ed25519 signature: recipient_public_key.verify(payload.as_bytes(), &signature)
4. If verification FAILS: log warning, discard. ❌ Do NOT store.
5. If hash not in workspace catalog: discard.
6. If recipient_node_id == seeder_node_id: discard (self-dealing, cannot verify).
7. Store verified receipt in seeding_receipts table.
```

**Receipt-based governance weight (hook only — do NOT enforce yet):**
```rust
// When tallying a node's votes, compute their receipt weight:
// SELECT COUNT(*) FROM seeding_receipts
//   WHERE seeder_node_id = voter_node_id
//     AND issued_at > unixepoch() - 1209600  -- last 14 days
//     AND recipient_node_id != voter_node_id  -- exclude self-dealing
//
// TODO(governance): When receipt-based governance is enforced, nodes with 0 verified receipts
// within 14 days will have no vote weight. For now, log this count alongside votes but do NOT
// reject votes based on receipt count.
```

**Lifecycle pruning worker (`pruning.rs`):**
```
Worker loop: runs once every 86400 seconds (once per day).
Add random jitter of 0–3600 seconds on first run to stagger all nodes.

Step A — Update last_any_seed_seen_at:
  UPDATE local_blobs SET last_any_seed_seen_at = unixepoch()
  WHERE hash IN (
      SELECT DISTINCT hash FROM file_seeds WHERE last_seen_at > unixepoch() - 3600
  );

Step B — Deprioritize old files (> 5 years = 157,766,400 seconds):
  Identify: SELECT hash FROM local_blobs WHERE catalog_first_seen_at < unixepoch() - 157766400
  Action: Mark these blobs as low-priority in the scarcity replication queue.
  Implementation: add an internal boolean flag or update a priority column.
  These files are placed LAST in the replication queue, same as downvoted files.
  ❌ Do NOT delete them. ❌ Do NOT evict them. Only deprioritize.

Step C — Purge dead files (0 seeds for >= 2 years = 63,072,000 seconds):
  SELECT hash, name FROM local_blobs
  WHERE is_pinned = 0
    AND (
      last_any_seed_seen_at < unixepoch() - 63072000
      OR (last_any_seed_seen_at IS NULL AND catalog_first_seen_at < unixepoch() - 63072000)
    );

  For each dead file found:
    a. If is_pinned = 1: SKIP — pinned files are NEVER pruned.
    b. Log: tracing::info!("Pruning dead file {} ({:?}) — 0 seeds for 2+ years", hash, name);
    c. DELETE FROM local_blobs WHERE hash = ?
    d. Attempt delete from iroh-blobs store (may already be evicted).
    e. Broadcast SyncMessage::FileDeleted { hash, deleter_node_id: our_node_id }
       so peers also remove the catalog entry.
```

**Rules:**
- ❌ Do NOT prune pinned blobs under any circumstances.
- ❌ Do NOT trust self-issued receipts (recipient must differ from seeder).
- ❌ Do NOT enforce receipt-based vote weighting yet — log only. Leave `TODO(governance)` marker.
- ✅ Ed25519 signature verification is mandatory before storing any receipt from a peer.

**Acceptance criteria:**
- [ ] Unit test: receipt signed by correct recipient key → stored successfully.
- [ ] Unit test: receipt with tampered/invalid signature → discarded, not stored.
- [ ] Unit test: receipt where `recipient_node_id == seeder_node_id` → discarded.
- [ ] Unit test: file with `last_any_seed_seen_at` > 2 years ago and `is_pinned = 0` → purged by worker.
- [ ] Unit test: pinned file with 0 seeds for 3 years → NOT pruned (is_pinned check).
- [ ] Unit test: file with `catalog_first_seen_at` > 5 years → placed last in scarcity replication queue.
- [ ] `TODO(governance)` comment exists in the vote weighting code at the receipt count enforcement hook.

---

## 12. Phase 2 — Milestones & Acceptance Criteria

| Milestone | Acceptance Criteria |
|---|---|
| **M7: Campus Stealth** | App detects `eduroam` SSID and disables DHT/mDNS within 35 seconds; TelemetryBar shows campus badge; lecture throttle activates at 09:00 and deactivates at 17:00. |
| **M8: Onboarding** | First launch shows wizard; settings persisted to SQLite; Connection Doctor identifies relay-only scenario and shows advice text; subsequent launches skip wizard. |
| **M9: SSD Safe Cache** | 1-seed blob auto-replicated within 2 worker intervals; daily write budget blocks excess auto-fill; newly written blobs (< 24h) not evicted regardless of LRU order. |
| **M10: Sole Seed Alerts** | Sole seed banner appears and disappears correctly; quit dialog prevents accidental close when sole seeds exist; tray icon changes to amber. |
| **M11: Immutable Paths** | Duplicate-path upload rejected with clear error; offline conflict resolution renames the loser deterministically and broadcasts the rename; executable files require warning modal to download. |
| **M12: Moderation** | 5 quarantine reports within 1h triggers quarantine; empty folder deletion blocked when files exist; negatively rated files placed last in scarcity replication queue. |
| **M13: Resilient Transfers** | Interrupted 100 MB download resumes from checkpoint on relaunch (not from byte 0); offline banner appears and disappears correctly with peer count changes. |
| **M14: Backups & Updates** | Backup ZIP contains identity key and DB only; folder export reports missing blobs before writing any data; update notification appears as startup toast and shutdown dialog only. |
| **M15: Receipt Governance** | Verified receipts stored in DB; invalid/self-dealing signatures rejected; dead files (0 seeds, 2+ years) pruned by worker; pinned files immune to pruning; `TODO(governance)` marker present. |

---

*Last updated: 2026-11-04 — Phase 2 fully detailed with implementation-ready specifications.*
