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
| 2026-11-04 | **Comprehensive Architecture & UX Alignment**: Finalized design specs for anchorless proof-of-storage, scarcity replication, SSD protection, Eduroam stealth mode, strict immutability, connection doctor, and onboarding wizard | Architectural grilling |
| 2026-11-05 | **Cross-Platform PC Autostart & Custom Cache Location**: Added `tauri-plugin-autostart` (Windows, Linux, macOS) and configurable blob storage directory | User requirement |
| 2026-11-05 | **ZIP Import Warning Modal & Toolbar Duplication Fix**: Added modal warning for unpacking ZIP archives with single-file alternative; removed duplicate empty folder toolbar buttons and fixed Back button shrink | Bug fix & UX |
| 2026-11-08 | **Auto-Updater Pipeline Fix & NSIS Prioritization (v0.1.42)**: Cleared persistent update dismissal on launch; prioritized NSIS setup packages; automated background update polling | Bug fix |
| 2026-11-08 | **SQLite WAL Truncation, Hardened Anti-Resurrection Tombstones & Bandwidth Settings (v0.1.43)**: Enforced periodic SQLite WAL checkpointing shrinking 4GB WAL; added dedicated Bandwidth & Battery tab; hardened recursive folder deletion against peer resurrection | Bug fix & UX |
| 2026-11-13 | **Public Safety Iteration 1 (v0.1.51)**: Webview capability minimization (removed raw `shell`/`fs`), Windows Mark-of-the-Web (`Zone.Identifier`), Unicode Bidi/RTLO path sanitization, macro/web extension lockout, and PDF JavaScript quarantine | Public Safety |
| 2026-11-13 | **Public Safety Iteration 2 (v0.1.52)**: Zero-Trust Auto-Hosting Guard (prevents blind public cache auto-fill/scarcity), Content Blocklist & Instant Quarantine (`blocked_blobs` with physical disk wipe), "Block & Purge Locally" context action, and P2P sync stream rate limiting (40 streams/10s, 500-item batch cap) | Public Safety |
| 2026-11-13 | **Public Safety Iteration 3 (v0.1.53)**: Public Swarm Privacy Disclosures & Warnings, "Privacy Shield / Stealth Mode" P2P relay encapsulation & IP hiding, Sybil-Resistant Deletion Immunity (protecting pinned files from peer votes), and dual Decimal GB / Binary GiB cache telemetry metrics | Public Safety |
| 2026-11-13 | **Build Fix for PDF.js Options (v0.1.54)**: Fixed TypeScript compilation failure in `PdfViewer.tsx` where `disableScripting` was not recognized in `DocumentInitParameters`; replaced with `isEvalSupported: false` and `enableXfa: false` | CI / Build Fix |
| 2026-11-13 | **Public Safety Iteration 4 (v0.1.55)**: Community Quarantine Enforcement, Threat Reporting Reasons & Early Warning Badges, Backend Execution Locks, and Anti-Masquerading Binary Magic-Byte Detection | Public Safety |

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
| R1.13 | Cross-Platform PC Autostart: Start application automatically on system boot (Windows Registry Run, Linux XDG Autostart, macOS LaunchAgent) | Should | ✅ |

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
| R3.12 | **Configurable Cache Storage Directory**: Custom disk directory on secondary drives or external SSDs for blob storage | Should | ✅ |

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
| R5.5 | Per-session egress cap: 150 KB/s (staged in `SessionLimiter`; Linux `tc` traffic control recommended on VPS) | Must | ✅ |
| R5.6 | Aggregate egress cap: 2 MB/s (staged in `SessionLimiter`; Linux `tc` traffic control recommended on VPS) | Must | ✅ |
| R5.7 | Token-bucket rate limiter staged module in `anchor.rs` | Must | ✅ |
| R5.8 | CLI args: --data-dir, --relay-port, --cert-dir, --max-session-bps, --max-aggregate-bps, --workspace-id | Must | ✅ |
| R5.9 | Workspace ID is optional namespace filter (defaults to shared campus swarm) | Should | ✅ |

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

---

## R14 — Anchorless Proof-of-Storage & Uptime Governance

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R14.1 | Bilateral Seeding Receipts: Peers issue and sign reciprocal transfer receipts for valid workspace BLAKE3 hashes when exchanging chunks (`anchor_voucher.rs`) | Must | ✅ |
| R14.2 | Sybil Defense via Multi-Peer Receipts: Voting weight and governance actions require verified receipts across valid catalog files | Must | ✅ |
| R14.3 | Anchor Attestation Hook: When an anchor daemon exists, it issues signed uptime/seeding vouchers to verify voting eligibility (`anchor_voucher.rs`) | Should | ✅ |
| R14.4 | Campus Email / SSO OTP (Phase 3 Deferred): Modular authentication interface allowing campus email verification via zero-domain Gmail SMTP relay | Could | ⏳ |

---

## R15 — Smart Scarcity Replication & SSD Wear Protection

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R15.1 | Scarcity-First Auto-Replication: Background worker auto-downloads endangered files with only 1 online seed to guarantee survival (`scarcity.rs`) | Must | ✅ |
| R15.2 | Abundance-Based Eviction: Cache eviction prioritizes files with $\ge 3$ verified peer seeds (highest seed count evicted first) | Must | ✅ |
| R15.3 | SSD Minimum Dwell Time: Recently written/downloaded blobs cannot be evicted within 24–48 hours of write | Must | ✅ |
| R15.4 | Daily Write Budget: Configurable daily SSD write ceiling (e.g. 2–5 GB/day) preventing background cache churn | Must | ✅ |
| R15.5 | In-Memory Chunk Buffering & Integrity: RAM buffer for chunk ingestion with BLAKE3 tree-hash verification and corrupt chunk auto-repair | Must | ✅ |
| R15.6 | Background Throttle Interval: Background scarcity downloads enforce a pacing delay (default 10 minutes, configurable) | Should | ✅ |

---

## R16 — Sole Seed Warning & Desktop Presence

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R16.1 | Floating Banner Alert: Dismissible top banner appears when local node is sole seed: 'Classmates rely on you for rare files. Keep app running to seed?' (`SoleSeedBanner.tsx`) | Must | ✅ |
| R16.2 | Taskbar Glow / Flash: Flash taskbar application icon on disconnect or shutdown attempt when hosting unique swarm files | Should | ✅ |
| R16.3 | Tray Icon State Alert: Highlight or animate system tray icon when actively preserving rare course materials | Should | ✅ |

---

## R17 — Campus Wi-Fi, Eduroam & Stealth Management

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R17.1 | Auto-SSID Detection: Automatically detect connection to `eduroam`, `campusroam`, or known university Wi-Fi networks (`network_profile.rs`) | Must | ✅ |
| R17.2 | Campus Stealth Mode: On campus networks, disable public BitTorrent DHT and mDNS; route exclusively through direct/relay QUIC TLS 443 | Must | ✅ |
| R17.3 | DPI Traffic Camouflage: Encapsulate transport to mimic standard HTTPS traffic on port 443 to resist deep packet inspection | Should | ✅ |
| R17.4 | Unrecognized Network Prompt: Discreet notification on new SSID asking user to classify as 'Personal / Private' vs 'Public / Campus' | Should | ✅ |
| R17.5 | SSID Management Rules: Settings table allowing users to view, add, and override network classifications and auto-rules | Should | ✅ |
| R17.6 | Scheduled Lecture Throttling: Automatically throttle background traffic during peak campus lecture hours (9 AM – 5 PM) | Should | ✅ |
| R17.7 | LEDBAT Adaptive Congestion Backoff: Automatically reduce background bandwidth when network latency or ping rises | Must | ✅ |
| R17.8 | User Priority Tiering: Manual user actions (downloads, reading, previews) always take absolute precedence over background swarm tasks | Must | ✅ |

---

## R18 — Strict File Immutability & Conflict Resolution

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R18.1 | Strict Absolute Lock: Virtual paths mapped to a hash are immutable while the record exists; updating requires explicit deletion and re-upload | Must | ✅ |
| R18.2 | Offline Split-Brain Resolution: If identical paths are created while disconnected, the earlier Unix timestamp wins; the loser is renamed with uploader suffix | Must | ✅ |
| R18.3 | Executable Sandbox Lockout: In-app execution of `.exe`, `.bat`, `.cmd`, `.sh`, `.ps1`, `.msi` is strictly prohibited; downloads require explicit warning modal | Must | ✅ |
| R18.4 | Cross-Platform Path Normalization: Globally enforce Windows-safe naming (no `<>:"/\\|?*` or reserved names) and case-insensitive uniqueness across all OSes (`path_utils.rs`) | Must | ✅ |

---

## R19 — Workspace Governance & Empty Folder Rules

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R19.1 | Single Program Workspace with Course Folders: Default to one shared workspace per academic cohort with root folders per course | Must | ✅ |
| R19.2 | Profile Switching: Support multiple workspace secret profiles for students participating in multiple independent swarms | Should | ✅ |
| R19.3 | Empty Folder Deletion Only: Folders have no individual owner; any student can create folders, but folders can ONLY be deleted when completely empty | Must | ✅ |

---

## R20 — Study Material Discovery, Ratings & Emergency Reporting

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R20.1 | Star Bookmarking: Allow students to star / favorite study files locally for quick access | Should | ✅ |
| R20.2 | Quality Upvotes / Downvotes: Swarm ratings displayed in UI to highlight high-quality course files and identify low-quality uploads | Should | ✅ |
| R20.3 | Downvote Seeding Demotion: Files with net negative ratings have background seeding priority reduced to save swarm cache space | Should | ✅ |
| R20.4 | Emergency Takedown Fast-Track: $\ge 5$ harmful/illegal reports within 1 hour marks file red as 'Dangerous' with 1-week quarantine | Must | ✅ |
| R20.5 | Emergency Quarantine Counter-Vote: Removing 'Dangerous' status requires a supermajority ($\ge 80\%$) counter-vote; anchor admin can unilaterally remove | Must | ✅ |

---

## R21 — Onboarding Experience & Diagnostics

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R21.1 | Dual-Mode Onboarding Wizard: First-run screen offering 'Simplified (Quick Setup)' vs 'Advanced Setup' (`OnboardingWizard.tsx`) | Must | ✅ |
| R21.2 | Modular Credentials Flow: Passphrase setup designed for seamless upgrade to university email OTP verification (`CredentialsStep.tsx`) | Must | ✅ |
| R21.3 | Built-in Connection Doctor: In-app diagnostic testing local mDNS, QUIC UDP reachability, relay latency, with green/red checklist & advice (`ConnectionDoctor.tsx`) | Must | ✅ |

---

## R22 — Resilient Transfers & Offline Experience

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R22.1 | Persistent Resume Queue: SQLite stores in-progress transfer chunk offsets across app restarts or laptop lid closes (`UploadQueueDrawer.tsx`) | Must | ✅ |
| R22.2 | Offline Header Banner: Prominent banner 'Offline Mode — viewing cached files only' with manual 'Retry Connection' button (`OfflineBanner.tsx`) | Must | ✅ |
| R22.3 | Available Offline Filter: 1-click header toggle filtering the file explorer to only cached and pinned local files | Should | ✅ |
| R22.4 | Visual Cloud Badges: Uncached remote-only files display a distinct cloud icon and dimmed row styling when offline | Should | ✅ |

---

## R23 — Export, Backups & Auto-Updates

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R23.1 | Full App Data Backup: Export single backup bundle (identity key, SQLite DB, pinned metadata) in Settings (`backup_cmds.rs`, `BackupSettings.tsx`) | Should | ✅ |
| R23.2 | Folder ZIP Export: 1-click folder export to local ZIP, prompting to download missing peer-only chunks before archiving (`ZipImportModal.tsx`) | Must | ✅ |
| R23.3 | Non-Intrusive Auto-Updater: Tauri update notifications prompted strictly on startup or shutdown to prevent study interruptions | Must | ✅ |

---

## R24 — Swarm Lifecycle & Obsolete File Pruning

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R24.1 | 5-Year Seeding Deprioritization: Files older than 5 years automatically have background auto-fill priority reduced (`pruning.rs`) | Should | ✅ |
| R24.2 | 2-Year Dead File Purge: Files with 0 online seeds continuously for 2 years are automatically purged from the workspace catalog (`pruning.rs`) | Must | ✅ |

---

## R7 — Dependency Matrix

```toml
[dependencies]
tauri          = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-fs    = "2"
tauri-plugin-autostart = "2.0.0-rc.1"
zip            = { version = "9.3.0", default-features = false, features = ["deflate"] }
iroh           = "1.4"
iroh-blobs     = "0.104"
iroh-relay     = { version = "1.4", features = ["server"] }
iroh-tickets   = "1.0"
tokio          = { version = "1", features = ["full"] }
rusqlite       = { version = "0.31", features = ["bundled", "backup"] }
hex            = "0.4"
serde          = { version = "1", features = ["derive"] }
serde_json     = "1"
chrono         = { version = "0.4", features = ["serde", "clock"] }
url            = "2"
semver         = "1.0"
reqwest        = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
anyhow         = "1"
thiserror      = "2"
dirs           = "7"
rfd            = "0.17"
clap           = { version = "4", features = ["derive"] }
tracing        = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
governor       = "0.7"
```

---

*Last updated: 2026-11-07 — updated with phase 2 completion, locked cargo dependencies, and lightweight anchor configuration.*
