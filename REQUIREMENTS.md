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
| 2026-11-02 | Confirmed Svelte/React choice deferred to implementation | User prompt |
| 2026-11-02 | Frontend implementation verified: Vitest (9/9 tests pass), Vite build (100% pass) | Implementation |
| 2026-11-02 | Rust backend & anchor binary implemented; .gitignore & CI workflows added | Implementation |

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
| R1.10 | Right-click context menu: Pin, Download, Preview, Copy Hash | Should | ✅ |

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
| R2.7 | Peers are discovered via relay registration (no mDNS required) | Should | ✅ |
| R2.8 | Peer count shown in UI telemetry footer | Should | ✅ |

---

## R3 — Storage & Eviction Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R3.1 | SQLite tracks every stored blob (hash, size, last_accessed_at, is_pinned) | Must | ✅ |
| R3.2 | Eviction triggers at 90% of quota | Must | ✅ |
| R3.3 | Eviction frees down to 80% of quota | Must | ✅ |
| R3.4 | Eviction removes oldest-first (LRU by last_accessed_at) | Must | ✅ |
| R3.5 | Pinned blobs are NEVER evicted | Must | ✅ |
| R3.6 | Eviction runs async (background worker, not blocking UI) | Must | ✅ |
| R3.7 | Eviction check runs every 60 seconds AND on every blob write | Must | ✅ |
| R3.8 | `last_accessed_at` updated every time a blob is opened/read | Must | ✅ |
| R3.9 | SQLite migrations versioned (0001, 0002, ...) | Should | ✅ |
| R3.10 | Settings stored in SQLite `settings` table (not a config file) | Should | ✅ |

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
| R4.8 | Cache quota slider: 3 GB → 50 GB + Full Mirror toggle | Must | ✅ |
| R4.9 | Large file streaming via Tauri events (not single IPC call) | Must | ✅ |
| R4.10 | Tech stack: React or Svelte + TailwindCSS + Lucide Icons + Vite | Must | ✅ |

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

## R6 — Docker / Deployment Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R6.1 | `anchor` binary runs natively without Docker (systemd-friendly) | Must | ✅ |
| R6.2 | `--no-relay` flag makes the same binary act as a pure storage node (no relay started) | Must | ✅ |
| R6.3 | `docker/Dockerfile` — one image; covers relay+node and node-only via `--no-relay` | Should | ✅ |
| R6.4 | `docker/docker-compose.yml` — single service, single container; `docker compose up -d` is sufficient | Should | ✅ |
| R6.5 | `docker/.env.example` — documents all required env vars | Should | ✅ |
| R6.6 | Persistent volume for blob storage | Must | ✅ |
| R6.7 | Read-only mount for /etc/letsencrypt | Must | ✅ |
| R6.8 | Expose 443/tcp, 443/udp; optionally 80/tcp for certbot | Must | ✅ |
| R6.9 | `restart: unless-stopped` (Docker) / `Restart=on-failure` (systemd) | Should | ✅ |
| R6.10 | Multi-stage Docker build (builder + minimal runtime image) | Should | ✅ |
| R6.11 | `systemd` unit file provided as an alternative to Docker | Should | ✅ |

---

## R7 — Security Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R7.1 | BLAKE3 verification on every received chunk (via iroh-blobs, not custom) | Must | ⬜ |
| R7.2 | Path traversal prevention on all user-supplied paths | Must | ⬜ |
| R7.3 | No unauthenticated HTTP/RPC ports on student machines | Must | ⬜ |
| R7.4 | SecretKey owner-only file permissions (0o600 / Windows ACL) | Must | ⬜ |
| R7.5 | Relay TLS required (no plaintext relay connections) | Must | ⬜ |
| R7.6 | IPC commands sanitize all string inputs before use as paths | Must | ⬜ |

---

## R8 — Testing Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| R8.1 | Unit tests for LRU eviction (oldest-first, pin survival, 80% target) | Must | ⬜ |
| R8.2 | Unit tests for path traversal rejection | Must | ⬜ |
| R8.3 | Unit test: SecretKey serialization round-trip | Must | ⬜ |
| R8.4 | Integration test: two local Iroh nodes exchange a blob | Must | ⬜ |
| R8.5 | Frontend component tests (Vitest) with mocked IPC | Should | ⬜ |
| R8.6 | CI: build both Windows and Linux targets on push to main | Must | ⬜ |
| R8.7 | CI: clippy with -D warnings (zero warnings policy) | Must | ⬜ |
| R8.8 | CI: cargo fmt check | Must | ⬜ |

---

## R9 — Dependency Version Requirements

> **IMPORTANT for agents:** Pin these exact versions. Iroh is pre-1.0 and has breaking API changes between minor versions. Do NOT upgrade without checking the changelog.

```toml
# src-tauri/Cargo.toml — required dependency versions
# Agent: run `cargo search iroh` to find the latest stable and use it,
# then LOCK it here. Do not leave these as wildcards.

[dependencies]
iroh          = "0.x.y"   # FILL IN: latest stable at time of implementation
iroh-blobs    = "0.x.y"   # must match iroh version compatibility matrix
tokio         = { version = "1", features = ["full"] }
rusqlite      = { version = "0.31", features = ["bundled"] }
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
anyhow        = "1"
dirs          = "5"
clap          = { version = "4", features = ["derive"] }   # anchor CLI args
tracing       = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile      = "3"        # for eviction tests using temp directories
tokio-test    = "0.4"
```

> **Note for agents:** After running `cargo add iroh`, copy the resolved version from `Cargo.lock` into `REQUIREMENTS.md` here so future sessions know exactly what version was used.

**Resolved versions (fill in after first `cargo build`):**
- iroh: *(not yet resolved)*
- iroh-blobs: *(not yet resolved)*

---

## R10 — Design Decisions Log

> Record every significant design decision here so future agents don't re-debate closed questions.

| Decision | Choice Made | Rationale | Date |
|---|---|---|---|
| Frontend framework | React OR Svelte (defer to implementer) | Both work with Vite + Tauri; Svelte is lighter | 2026-11-02 |
| State management | Tauri `State<T>` + `Arc<RwLock<>>` | Tauri's built-in state avoids extra deps | 2026-11-02 |
| PDF library | `pdfjs-dist` (preferred) | More actively maintained than react-pdf-viewer | 2026-11-02 |
| Markdown editor | `@monaco-editor/react` or Milkdown | Monaco is heavier but more familiar | 2026-11-02 |
| Rate limiter style | Token bucket (not leaky bucket) | Token bucket allows short bursts; simpler to implement | 2026-11-02 |
| Docker strategy | One binary (`anchor`), Docker optional; single container covers relay+node via `--no-relay` flag | Simplest possible VPS setup; no compose orchestration needed | 2026-11-02 |
| SecretKey storage | `dirs::data_dir()/p2pstud/iroh_secret.key` | Platform-standard; hidden from casual file browsing | 2026-11-02 |
| SQLite bundling | `rusqlite` with `bundled` feature | No system SQLite dependency required | 2026-11-02 |
| Eviction check frequency | Every 60s + on every blob write | Balance between responsiveness and CPU overhead | 2026-11-02 |

---

## R11 — Open Questions

> Questions that have NOT yet been answered. An agent that resolves one should move it to R10 with the decision made.

| # | Question | Raised | Notes |
|---|---|---|---|
| Q1 | Which frontend framework — React or Svelte? | 2026-11-02 | Either works; decide at Step 8 |
| Q2 | How is the workspace namespace ID distributed to students? | 2026-11-02 | Invite link? QR code? Config file? |
| Q3 | Is mDNS/local peer discovery needed (in addition to relay)? | 2026-11-02 | Useful for LAN speed but may conflict with campus policies |
| Q4 | What is the workspace file namespace model? Flat or hierarchical? | 2026-11-02 | The file explorer implies hierarchy — needs a manifest blob |
| Q5 | Should the anchor node support multiple workspace IDs simultaneously? | 2026-11-02 | Single for MVP, multi-workspace as v2 feature |
| Q6 | Auto-update mechanism for the desktop app? | 2026-11-02 | Tauri updater plugin available |
| Q7 | What is the exact iroh API for starting a relay server? (verify against docs) | 2026-11-02 | Agent must check iroh source/examples before implementing |

---

*Last updated: 2026-11-02 — initial creation from user session.*
