# AGENTS.md — Subagent Roles & Operating Instructions

> **Read this file** before spawning or acting as any subagent. Every agent in this project has a defined scope, file ownership, and communication contract. Staying inside your scope prevents merge conflicts and duplicate work.

---

## How the Orchestrator Works

The **OrchestratorAgent** (the top-level agent in the session) is responsible for:
1. Reading `AGENT_PLAN.md` → determining what is ✅ done, 🔄 in-progress, ⬜ pending, ❌ blocked.
2. Spawning subagents for parallel work streams.
3. Merging outputs, resolving conflicts, running integration tests, and committing.
4. Updating the Progress Tracker in `AGENT_PLAN.md` after each step.
5. Updating `REQUIREMENTS.md` if new decisions are made.

**Spawn subagents in parallel when steps have no cross-dependencies.** For example, Steps 8–12 (frontend) can run in parallel with Steps 5–7 (SQLite eviction), because they touch completely different files.

---

## Agent Roster

### 1. OrchestratorAgent

| Field | Value |
|---|---|
| **Role** | Top-level coordinator and integration agent |
| **Spawned by** | User / session start |
| **Owns** | `AGENT_PLAN.md`, `REQUIREMENTS.md`, `AGENTS.md`, `Cargo.toml` (workspace root), final `git commit` |

**Startup checklist (run every session):**
- [ ] Read `AGENT_PLAN.md` — check Progress Tracker for current state.
- [ ] Read `REQUIREMENTS.md` — check for any new requirements since last session.
- [ ] Run `cargo check --workspace` if code exists — note any compile errors before assigning work.
- [ ] Identify which steps are unblocked and can be parallelized.
- [ ] Spawn appropriate subagents (see roster below).
- [ ] After all subagents complete, run `cargo test --all` and `npm run test`.
- [ ] Update Progress Tracker and commit.

---

### 2. RustBackendAgent

| Field | Value |
|---|---|
| **Role** | Implements all Rust backend code inside `src-tauri/src/` |
| **Spawned by** | OrchestratorAgent |
| **Plan steps** | 2, 3, 4, 5, 6, 7 |
| **Owns (write)** | `src-tauri/src/p2p/`, `src-tauri/src/storage/`, `src-tauri/src/commands/`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` |
| **Must NOT touch** | `src/` (frontend), `src-tauri/src/bin/anchor.rs`, `docker/` |

**Startup instructions:**
1. Read `AGENT_PLAN.md` sections 4A and 4B fully.
2. Read `REQUIREMENTS.md` section "Backend Requirements".
3. Check `src-tauri/Cargo.toml` — verify `iroh`, `iroh-blobs`, `rusqlite`, `tokio` are present.
4. Before writing code, run `cargo check -p p2pstud-tauri` to see current compiler state.

**Completion criteria:**
- `cargo check -p p2pstud-tauri` passes with zero errors.
- `cargo test -p p2pstud-tauri` passes all unit tests.
- All IPC commands in `commands/` are registered in `main.rs` via `.invoke_handler(tauri::generate_handler![...])`.
- Report back to OrchestratorAgent with: list of implemented commands, any TODOs left, compiler warnings to address.

**Common mistakes to avoid:**
- ❌ Do NOT use `std::sync::Mutex` around async code — use `tokio::sync::Mutex`.
- ❌ Do NOT call `block_on` inside a Tauri command — all commands must be `async fn`.
- ❌ Do NOT store the Iroh `Node` in a `Mutex<Option<Node>>` and unwrap unsafely — use `Arc<RwLock<Option<IrohEngine>>>` with a proper error if not initialized.
- ✅ Map all `anyhow::Error` to `String` at the IPC boundary: `.map_err(|e| e.to_string())`.

---

### 3. FrontendAgent

| Field | Value |
|---|---|
| **Role** | Implements the Svelte/React frontend |
| **Spawned by** | OrchestratorAgent |
| **Plan steps** | 8, 9, 10, 11, 12 |
| **Owns (write)** | `src/` (all frontend source), `index.html`, `vite.config.ts`, `package.json`, `tailwind.config.js` |
| **Must NOT touch** | `src-tauri/`, `docker/`, `Cargo.toml` |

**Startup instructions:**
1. Read `AGENT_PLAN.md` section 4D fully.
2. Read `REQUIREMENTS.md` section "Frontend Requirements".
3. Run `npm install` if `node_modules` is absent.
4. Run `npm run dev` to verify the dev server starts (Tauri not required for UI-only dev).

**Completion criteria:**
- `npm run build` exits with zero errors.
- `npm run test` (Vitest) passes all component tests.
- The file explorer renders a mock file tree with all four sync-state badges.
- PDF viewer loads a local test PDF without errors.
- Markdown editor shows split-pane preview on input.
- Telemetry footer polls `get_storage_stats` mock and displays correctly.
- Report back to OrchestratorAgent with: components implemented, any IPC commands called (so backend agent can verify they exist).

**Common mistakes to avoid:**
- ❌ Do NOT use `fetch()` or `XMLHttpRequest` to talk to Tauri — always use `@tauri-apps/api/core` invoke.
- ❌ Do NOT hardcode file paths — all paths come from Tauri IPC responses.
- ✅ For large file streaming, listen for Tauri events (`listen('file-chunk', ...)`) rather than awaiting one large `invoke`.
- ✅ Mock all Tauri IPC calls in tests using `vi.mock('@tauri-apps/api/core')`.

---

### 4. AnchorDaemonAgent

| Field | Value |
|---|---|
| **Role** | Implements the headless VPS anchor binary and all Docker infrastructure |
| **Spawned by** | OrchestratorAgent |
| **Plan steps** | 13, 14, 15 |
| **Owns (write)** | `src-tauri/src/bin/anchor.rs`, `docker/Dockerfile.anchor`, `docker/Dockerfile.node`, `docker/docker-compose.yml`, `docker/docker-compose.single.yml`, `docker/.env.example` |
| **Must NOT touch** | `src-tauri/src/` (non-bin), `src/` |

**Startup instructions:**
1. Read `AGENT_PLAN.md` sections 4C and 8 fully.
2. Read `REQUIREMENTS.md` section "Anchor / Docker Requirements".
3. Check that shared code (e.g., `p2p/identity.rs`) is available to import as a library from `anchor.rs`.
4. The anchor binary must compile with: `cargo build --release --bin anchor`.

**Completion criteria:**
- `cargo build --release --bin anchor` succeeds.
- `docker build -f docker/Dockerfile.anchor .` succeeds.
- `docker build -f docker/Dockerfile.node .` succeeds (headless node variant).
- `docker compose -f docker/docker-compose.single.yml up --dry-run` succeeds.
- Rate limiter unit test: simulate 10 sessions each requesting 1 MB/s → assert aggregate stays ≤ 2 MB/s.
- Report back to OrchestratorAgent with: CLI flags implemented, Docker image sizes, any TLS setup steps remaining.

---

### 5. TestingAgent

| Field | Value |
|---|---|
| **Role** | Writes and runs all tests; owns CI workflow files |
| **Spawned by** | OrchestratorAgent (after RustBackendAgent completes Steps 2–7) |
| **Plan steps** | 16, 17, 18 |
| **Owns (write)** | `src-tauri/tests/`, `src-tauri/src/**/tests.rs` (inline test modules), `.github/workflows/` |
| **Must NOT touch** | Production source files (only add `#[cfg(test)]` blocks, never modify non-test logic) |

**Startup instructions:**
1. Read `AGENT_PLAN.md` sections 5 and 6 fully.
2. Run `cargo test --all 2>&1` — record the current pass/fail state as a baseline.
3. Identify which tests from the plan are not yet written.

**Required test cases (must all pass before Step 16 is marked ✅):**

```
[ ] eviction::tests::eviction_removes_oldest_first
[ ] eviction::tests::pinned_blobs_survive_eviction
[ ] eviction::tests::eviction_target_is_80_percent
[ ] eviction::tests::full_mirror_skips_eviction
[ ] storage::tests::path_traversal_rejected
[ ] storage::tests::path_within_base_accepted
[ ] p2p::tests::secret_key_round_trips
[ ] integration::tests::two_nodes_transfer_blob  (feature-gated)
```

**Completion criteria:**
- `cargo test --all` passes with zero failures.
- `cargo clippy -- -D warnings` produces zero warnings.
- `.github/workflows/build.yml` and `test.yml` are present and syntactically valid YAML.
- Report back to OrchestratorAgent with: test count, any tests skipped and why.

---

### 6. DocsAgent

| Field | Value |
|---|---|
| **Role** | Writes `README.md`, inline doc comments, and the deployment guide |
| **Spawned by** | OrchestratorAgent (run last, after M5) |
| **Plan steps** | 19 |
| **Owns (write)** | `README.md`, `docs/` directory |
| **Must NOT touch** | Any `.rs` or `.ts` source files |

**Completion criteria:**
- `README.md` covers: project overview, prerequisites, local dev setup, VPS deployment, contributing guide.
- Every public Rust function has a `///` doc comment.
- `cargo doc --no-deps` builds without warnings.

---

## Inter-Agent Communication Protocol

When a subagent finishes, it **must report** to OrchestratorAgent with:

```
AGENT: <AgentName>
STEPS COMPLETED: <list of step numbers>
FILES WRITTEN: <list of file paths>
TESTS PASSING: <yes/no + count>
BLOCKERS: <none | description>
NEXT SUGGESTED ACTION: <what orchestrator should do next>
```

OrchestratorAgent then:
1. Verifies the files exist and `cargo check` still passes.
2. Updates the Progress Tracker in `AGENT_PLAN.md`.
3. Decides whether to spawn the next agent or continue itself.

---

## Parallelism Map

```
Session Start
    │
    ├─[parallel]─ RustBackendAgent (Steps 2–7)
    │               └─ signals done → TestingAgent picks up
    │
    ├─[parallel]─ FrontendAgent (Steps 8–12)
    │
    └─[parallel]─ AnchorDaemonAgent (Steps 13–15)
                    └─ signals done → OrchestratorAgent runs E2E tests (Step 16)

After all parallel streams done:
    OrchestratorAgent → Step 16 (E2E test) → Step 17 (security) → Step 18 (CI) → DocsAgent (Step 19)
```

---

*Last updated: 2026-11-02 — initial creation.*

