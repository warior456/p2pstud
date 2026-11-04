# REQUIREMENTS.md — Living Requirements Document

> **Purpose:** This file is the single source of truth for ALL requirements gathered across chat sessions, user feedback, and design decisions. Update it whenever a new decision is made or a requirement changes. When context-switching to a new agent session, read this file FIRST — it captures everything that might not be obvious from the code alone.

> **For agents:** After reading `AGENT_PLAN.md`, read this file before writing any code. If you make a design decision not covered here, ADD IT to the relevant section before finishing your session.

---

## Changelog

| Date | Change | Source |
|---|---|---|
| 2026-11-02 | Initial requirements captured from user specification | User prompt |
| 2026-11-02 | Added Docker single-node build requirement | User follow-up |
| 2026-11-02 | Added headless Docker node variant (no GUI, P2P only) | User follow-up |
| 2026-11-02 | **Clarified**: Docker is optional; anchor binary is primary artifact; single container suffices | User follow-up |
| 2026-11-02 | Frontend implementation verified: Vitest (9/9 tests pass), Vite build (100% pass) | Implementation |
| 2026-11-02 | Rust backend & anchor binary implemented; .gitignore & CI workflows added | Implementation |
| 2026-11-03 | Added system tray background persistence and close-to-tray window management | User goal |
| 2026-11-03 | Fixed Windows 10/11 `WebView2Loader.dll` dynamic link dependency packaging | User feedback |
| 2026-11-03 | Direct in-process blob ingestion avoiding flume RPC stream interruption | User bug report |
| 2026-11-03 | Full cross-platform Linux parity (`xdg-open`, `0o600` secret key, `.deb`/`.AppImage` CI) | User prompt |
| 2026-11-03 | Enabled multi-layer discovery (Local Swarm mDNS, n0 DNS pkarr, BitTorrent DHT) | User feedback |
| 2026-11-03 | Real-time workspace catalog sync over QUIC (`/p2pstud/sync/1`) & on-demand download | User feedback |
| 2026-11-03 | Orphaned file management & force cleanup when owner no longer exists | User requirement |
| 2026-11-03 | **Active Swarm Hosting Auto-Fill**: Cache actively fills to ~80% to host and seed unpinned course files | User requirement |
| 2026-11-03 | **Target Disk Bloat Elimination**: Stripped dependency debug symbols; added auto-pruning script | User requirement |
| 2026-11-03 | **Real-Time Bandwidth Telemetry**: Replaced static mock speeds with dynamic byte-rate meter | User bug report |
| 2026-11-03 | **Community Delete Voting / Consensus Moderation**: Prohibited non-owner deletion; added 3-vote quorum | User requirement |
| 2026-11-03 | **Upload Processing Indication**: Added live upload queue drawer & file row progress tracking | User requirement |
| 2026-11-03 | **Smart Least-Seeds Cache**: Auto-fill prioritizes replicating rare/endangered files with lowest seed count | User requirement |
| 2026-11-03 | **Hierarchical Folder Support**: Virtual folder tree, directory navigation, "New Folder", folder-aware uploads | User requirement |
| 2026-11-03 | **File Moving & Sole Seed Eviction Protection**: Drag/context file moving; block disk eviction if only seed | User requirement |
| 2026-11-04 | **Multi-Instance Fix & Single Instance Plugin**: Added `tauri-plugin-single-instance` and fixed DB forward-migration bounds check | Bug fix |
| 2026-11-04 | **Comprehensive Security Audit & Patching**: Path traversal defense, DOMPurify XSS mitigation, QUIC vote spoofing prevention, authorized file moves | Security audit |
| 2026-11-04 | **P2P Swarm Network Secret**: Workspace passphrase derives dynamic QUIC ALPN to isolate classroom swarms from public discovery | User requirement |
| 2026-11-04 | **Dev Mode ZIP Archive Import**: Fast bulk-importing of course packages in dev mode with Zip Slip directory traversal protection | User requirement |

---

## R1 — Core Product Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R1.1 | Desktop app for Windows and Linux | Must | ✅ |
| R1.2 | Idle RAM ≤ 120 MB | Must | ✅ |
| R1.3 | Default cache quota: 5 GB (min 3 GB, max Full Mirror) | Must | ✅ |
| R1.4 | Files are content-addressed by BLAKE3 hash | Must | ✅ |
| R1.5 | Works on Eduroam / symmetric NAT via TCP 443 relay fallback | Must | ✅ |
| R1.6 | No custom crypto — use Iroh's audited Ed25519 + BLAKE3 | Must | ✅ |
| R1.7 | No custom socket protocol — use Iroh's QUIC transport | Must | ✅ |
| R1.8 | Pin/unpin files for offline protection | Should | ✅ |
| R1.9 | Full Mirror mode disables eviction, auto-downloads all blobs | Should | ✅ |
| R1.10 | Right-click context menu: Pin, Download, Preview, Copy Hash, Delete | Should | ✅ |
| R1.11 | System tray icon with background persistence (close [X] hides to tray) | Must | ✅ |
| R1.12 | One shared cloud folder: all peers access workspace; users can only delete their own files | Must | ✅ |

---

## R2 — Networking & P2P Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R2.1 | Embed Iroh node in-process (no external daemon) | Must | ✅ |
| R2.2 | Node identity (SecretKey) persisted across restarts | Must | ✅ |
| R2.3 | SecretKey file permissions: owner-only (0o600 / Windows ACL) | Must | ✅ |
| R2.4 | Custom relay URL configurable (not hardcoded) | Must | ✅ |
| R2.5 | Direct QUIC attempted first, relay used as fallback | Must | ✅ |
| R2.6 | Relay must use TLS on port 443 | Must | ✅ |
| R2.7 | Multi-layer discovery: `LocalSwarmDiscovery` (LAN mDNS), `discovery_n0` (DNS), `discovery_dht` (DHT) | Must | ✅ |
| R2.8 | Peer count & status shown in UI telemetry footer | Should | ✅ |
| R2.9 | Smart peer dialing: accepts `NodeTicket` (`node...`), JSON `NodeAddr`, `node_id@address`, or raw hex | Must | ✅ |
| R2.10 | Background swarm maintenance loop: automatically connects to discovered local peers every 5s | Must | ✅ |
| R2.11 | Workspace catalog sync protocol over QUIC (`/p2pstud/sync/1`) exchanging file entries between peers | Must | ✅ |

---

## R3 — Storage, Eviction & Active Swarm Hosting

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R3.1 | SQLite tracks every stored blob (hash, size, last_accessed_at, is_pinned, owner_node_id, name) | Must | ✅ |
| R3.2 | Eviction triggers at 90% of quota | Must | ✅ |
| R3.3 | Eviction frees down to 80% of quota | Must | ✅ |
| R3.4 | Eviction removes oldest-first (LRU by last_accessed_at) | Must | ✅ |
| R3.5 | Pinned blobs are NEVER evicted | Must | ✅ |
| R3.6 | Eviction runs async (background worker, not blocking UI) | Must | ✅ |
| R3.7 | Eviction check runs periodically AND on blob writes | Must | ✅ |
| R3.8 | `last_accessed_at` updated every time a blob is opened/read | Must | ✅ |
| R3.9 | **Active Swarm Hosting Auto-Fill**: Cache actively fills to ~80% to host & seed unpinned files for the P2P swarm | Must | ✅ |
| R3.10 | **Orphaned File Management**: When a file's owner no longer exists, members can force-clean up the orphaned file | Must | ✅ |
| R3.11 | On-demand remote blob download: clicking/reading a cloud-only file downloads it from available peers | Must | ✅ |

---

## R4 — Frontend / UI Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R4.1 | File explorer with breadcrumb navigation | Must | ✅ |
| R4.2 | Four sync-state badges: Cloud Only, Cached/Seeding, Pinned, Downloading | Must | ✅ |
| R4.3 | Drag-and-drop file upload | Should | ✅ |
| R4.4 | PDF viewer (pdfjs-dist) — in-app, no browser tab | Must | ✅ |
| R4.5 | Markdown editor with split-pane live preview | Should | ✅ |
| R4.6 | Markdown auto-save with 1-second debounce | Should | ✅ |
| R4.7 | Telemetry footer: peer count, cache usage, upload/download speed | Must | ✅ |
| R4.8 | Cache & Swarm Settings tabs: Quota slider (3-50GB), Node Ticket, Connected Peers table, Purge cache | Must | ✅ |
| R4.9 | Real in-app file download & browser export | Must | ✅ |
| R4.10 | Orphaned file deletion modal confirmation | Must | ✅ |

---

## R5 — Anchor Node Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R5.1 | Headless Rust binary (`anchor`) — no GUI, no Tauri | Must | ✅ |
| R5.2 | Full mirror mode always enabled (no eviction) | Must | ✅ |
| R5.3 | Runs embedded iroh-net relay on configurable port (default 443) | Must | ✅ |
| R5.4 | TLS via Let's Encrypt certificates (path configurable) | Must | ✅ |
| R5.5 | Per-session egress cap: 150 KB/s (configurable) | Must | ✅ |
| R5.6 | Aggregate egress cap: 2 MB/s (configurable) | Must | ✅ |
| R5.7 | Token-bucket rate limiter (not leaky bucket) | Must | ✅ |
| R5.8 | CLI args: --data-dir, --relay-port, --cert-dir, --max-session-bps, --max-aggregate-bps, --workspace-id | Must | ✅ |
| R5.9 | Workspace ID is required (no default) | Must | ✅ |

---

## R6 — Packaging & Production Parity

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R6.1 | Windows installers: NSIS Setup (`.exe`) and MSI (`.msi`) | Must | ✅ |
| R6.2 | Windows Portable: ZIP archive bundling `p2pstud.exe` + `WebView2Loader.dll` | Must | ✅ |
| R6.3 | Linux packages: `.deb` and `.AppImage` via Tauri v2 bundling | Must | ✅ |
| R6.4 | Docker container for headless Linux anchor daemon (`debian:bookworm-slim`) | Must | ✅ |
| R6.5 | System file manager launcher (`xdg-open` on Linux, `explorer` on Windows) | Must | ✅ |
| R6.6 | CI workflow matrix testing and bundling on Windows and Ubuntu | Must | ✅ |

---


---

## R8 — Moderation, Ownership & Consensus Deletion

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R8.1 | Only the original file uploader can directly delete a file unilaterally | Must | ✅ |
| R8.2 | Non-owners CANNOT unilaterally delete files from the workspace | Must | ✅ |
| R8.3 | Non-owners can cast a vote to delete (vote_delete_file) or retract their vote (unvvote_delete_file) | Must | ✅ |
| R8.4 | Consensus quorum: files require 3 community votes before being purged automatically | Must | ✅ |
| R8.5 | Delete votes sync across all connected swarm peers over QUIC (FileVoteDelete, FileUnvoteDelete) | Must | ✅ |
| R8.6 | Non-owners can always free their own local disk space (evict_local_file) without touching cloud catalog | Must | ✅ |
| R8.7 | Dynamic bandwidth meter (BandwidthTracker) computes live instantaneous upload & download speeds | Must | ✅ |

---

## R9 — Upload Queue Visualization & Smart Least-Seeds Cache

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R9.1 | Live upload queue tracking: Display uploading files with progress, status indicator, and spinner in UI | Must | ✅ |
| R9.2 | Smart Cache Seed Prioritization: Auto-fill background worker prioritizes fetching files with the least number of seeds | Must | ✅ |
| R9.3 | Seed Count Visibility: Show active seed count on each file entry (`🌱 X seeds`) with swarm health indicators | Should | ✅ |
| R9.4 | Drag-and-drop & file picker integration: Seamlessly enqueue multiple files and update state live | Must | ✅ |

---

## R10 — Hierarchical Folder Support & Organization

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R10.1 | Virtual directory hierarchy: Parse file paths (e.g. `Course/Week1/notes.pdf`) into nested folder view | Must | ✅ |
| R10.2 | Breadcrumb navigation: Clickable breadcrumbs (`Shared Cloud > Course > Week 1`) to navigate depth | Must | ✅ |
| R10.3 | Create Folder: "New Folder" action in toolbar allowing users to create directory groupings | Should | ✅ |
| R10.4 | Folder-aware uploads: Dropping or selecting files while inside a folder places them inside that directory | Must | ✅ |
| R10.5 | Folder statistics: Display folder item count and aggregated size in the folder row | Should | ✅ |

---

## R11 — File Moving & Swarm Path Synchronization

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R11.1 | File moving: Allow users to move files between folders or to the workspace root | Must | ✅ |
| R11.2 | Drag-and-drop file moving: Dragging a file onto a folder row moves it into that folder | Should | ✅ |
| R11.3 | Context menu file moving: "Move to..." action with folder picker/prompt | Must | ✅ |
| R11.4 | Swarm synchronization: Broadcast `SyncMessage::FileMoved` across QUIC to update all connected nodes | Must | ✅ |

---

## R12 — Sole Active Seed Disk Eviction Protection

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R12.1 | Disallow disk eviction if sole active seed: `evict_local_file` returns error if remote seed count is 0 | Must | ✅ |
| R12.2 | Permanent loss prevention: Ensure un-replicated academic materials are never deleted from disk by accident | Must | ✅ |
| R12.3 | UI safety indication: Disable or guard "Free Local Space" with tooltip and alert explaining sole-seed role | Should | ✅ |

---

## R13 — Anti-Delete (Keep) Voting, 86% Ratio Enforcement & Grace Period

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R13.1 | Malicious Client Protection: Non-owners cannot delete files unilaterally across peers via `SyncMessage::FileDeleted`; only original verified owners can unilaterally remove their files | Must | ✅ |
| R13.2 | Anti-Delete (Keep) Voting: Swarm peers can cast and retract keep/defense votes (`file_keep_votes`, `SyncMessage::FileVoteKeep`, `SyncMessage::FileUnvoteKeep`) | Must | ✅ |
| R13.3 | 86% Vote Ratio Consensus: Community deletion requires $D \ge 3$ delete votes and a delete ratio of $\ge 86\%$ ($D / (D + K) \ge 0.86$) | Must | ✅ |
| R13.4 | 24-Hour Deletion Grace Period: When $\ge 86\%$ delete consensus is reached, the file is NOT immediately deleted; a 24-hour grace period (`deletion_scheduled_at`) begins | Must | ✅ |
| R13.5 | Grace Period Defense: If keep votes added during the grace period drop the delete ratio below 86%, the scheduled deletion is automatically aborted | Must | ✅ |
| R13.6 | Pending Deletion Warning: UI prominently displays workspace warning banners and row badges alerting students to files scheduled for deletion with time remaining and Defend action | Must | ✅ |

## R7 — Dependency Matrix

```toml
[dependencies]
iroh          = { version = "0.29", features = ["discovery-local-network", "discovery-pkarr-dht"] }
iroh-blobs    = "0.29"
iroh-relay    = "0.29"
tokio         = { version = "1", features = ["full"] }
rusqlite      = { version = "0.31", features = ["bundled"] }
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
anyhow        = "1"
dirs          = "5"
clap          = { version = "4", features = ["derive"] }
tracing       = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
governor      = "0.6"
```

---

*Last updated: 2026-11-03 — updated with active swarm hosting, peer discovery, catalog sync, and orphaned file management.*
