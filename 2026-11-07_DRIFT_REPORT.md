# Architecture & Specification Drift Report

**Generated:** 2026-11-07  
**Analysis Scope:** All Markdown specifications and design documents across root and `docs/` directories.

---

## docs/KULEUVEN_AUTH_SPEC.md

* **Missing Data Structures & Models:** `GroepTMemberTicket` struct and its cryptographic serialization/deserialization logic described in Section 4 are completely absent in `src-tauri/src/`.
* **Missing QUIC Transport Verification:** Peer mutual ticket challenge and the 3-point verification checklist (Authority Signature Check, Transport NodeId Binding Check, and Expiration Check) described in Section 5 are not implemented. QUIC synchronization in `src-tauri/src/p2p/engine.rs` and `src-tauri/src/p2p/sync.rs` is authenticated solely via passphrase-derived dynamic ALPN rather than individual student ticket credentials.
* **Missing OTP Issuance Flow:** The backend email verification flow (Flow A: Regex validation, OTP generation, and SMTP/API dispatch) and the `POST /api/auth/verify-otp` endpoint described in Section 6 are not implemented.
* **Missing Microsoft Entra ID Integration:** The OAuth2/OIDC redirection flow (Flow B) and the custom URI scheme handler `p2pstud://auth?ticket=...` are not implemented.
* **Missing Swarm CRL & Authority Key Rotation:** Swarm-wide Certificate Revocation List (CRL) broadcasting over QUIC sync streams and secondary authority key fallback described in Section 7 are absent.

## README.md

* **Anchor Egress Rate Limiting Disconnected:** Sections "Key Features" and "Headless Anchor Node Deployment" specify enforced token-bucket egress rate limiting (150 KB/s per session, 2 MB/s aggregate). While `SessionLimiter` is defined in `src-tauri/src/bin/anchor.rs:184-239`, an isolated instance is only ticked once with a dummy `consume(1)` call in a background task; it is not wired into the QUIC router or `BlobsProtocol` streams.
* **Unenforced Anchor Parameter `--workspace-id`:** In `src-tauri/src/bin/anchor.rs:49`, `workspace_id` is defined as `Option<String>` and is never validated, referenced, or used in `anchor.rs`, despite being documented as a required workspace flag.
* **Frontend Framework Ambiguity:** Documentation references Svelte alongside React in multiple places, whereas the application is implemented strictly in React 18 (`src/App.tsx`).

## REQUIREMENTS.md

* **Outdated Implementation Status for Requirements R14 through R24:** Requirements R14.1–R14.4 (Bilateral Seeding Receipts), R15.1–R15.6 (Smart Scarcity Replication & SSD Wear Protection), R16.1–R16.3 (Sole Seed Warning & Presence Alerts), R17.1–R17.8 (Campus Wi-Fi & Eduroam Stealth Mode), R18.1–R18.4 (Strict File Immutability & Conflict Resolution), R19.1–R19.3 (Course Folders & Empty Folder Deletion), R20.1–R20.5 (Study Ratings, Star Bookmarks & Emergency Quarantine), R21.1–R21.3 (Onboarding Wizard & Connection Doctor), R22.1–R22.4 (Persistent Transfer Resume Queue & Offline Mode), R23.1–R23.3 (Workspace Backup & Folder ZIP Export), and R24.1–R24.2 (Lifecycle Pruning) are marked as pending (`⬜`), but all have been implemented across `src-tauri/` and `src/`.
* **Dependency Matrix (R7) Version Drift:**
  * `iroh` is listed as version `0.29`, but `src-tauri/Cargo.toml` uses `1.4`.
  * `iroh-blobs` is listed as version `0.29`, but `src-tauri/Cargo.toml` uses `0.104`.
  * `iroh-relay` is listed as version `0.29`, but `src-tauri/Cargo.toml` uses `1.4`.
  * `dirs` is listed as version `5`, but `src-tauri/Cargo.toml` uses `7`.
  * `governor` is listed as version `0.6`, but `src-tauri/Cargo.toml` uses `0.7`.
  * Missing dependencies in R7 matrix that are present in `src-tauri/Cargo.toml`: `iroh-tickets = "1.0"`, `zip = "9.3.0"`, `chrono = "0.4"`, `url = "2"`, `semver = "1.0"`, `reqwest = "0.12"`, `thiserror = "2"`, `rfd = "0.17"`, and `tauri-plugin-autostart = "2.0.0-rc.1"`.
* **Anchor Egress Rate Limiting (R5.5–R5.7):** Requirements R5.5, R5.6, and R5.7 specify active egress throttling on the anchor node, but `src-tauri/src/bin/anchor.rs` does not attach `SessionLimiter` to the streaming data endpoints.
* **Mandatory Workspace ID (R5.9):** R5.9 states that Workspace ID is required (no default), but `src-tauri/src/bin/anchor.rs` treats it as optional and does not use it.

## AGENTS.md

* **Non-Existent Docker File Ownership:** Section "Agent Roster (AnchorDaemonAgent)" specifies write ownership over `docker/Dockerfile.anchor`, `docker/Dockerfile.node`, and `docker/docker-compose.single.yml`. These files do not exist in the repository; the project uses consolidated files `docker/Dockerfile`, `docker/docker-compose.yml`, and `docker/p2pstud-anchor.service`.
* **FrontendAgent Framework Scope:** Describes the role as "Implements the Svelte/React frontend", but frontend is purely React 18 with Vite and TypeScript.
* **Unchecked Test Checklist:** The required unit test cases checklist in Section 5 contains unchecked boxes (`[ ]`), although all listed unit tests (`eviction_removes_oldest_first`, `pinned_blobs_survive_eviction`, `eviction_target_is_80_percent`, `full_mirror_skips_eviction`, `path_traversal_rejected`, `path_within_base_accepted`, `secret_key_round_trips`) are implemented and passing.

## AGENT_PLAN.md

* **Progress Tracker Row Omission:** The header states that Step 53 completed addressing all 14 security and architectural findings from `SECURITY_AUDIT_REPORT.md` and bumped the version to `0.1.25`. However, row 53 in the Progress Tracker table records "Single-Store Architecture, Concurrent Redb Lock Fix & Ready-Polling Discovery ... bumped version to 0.1.24", leaving the security audit remediations without a dedicated row.
* **Incomplete Project File Map:** Section 3 ("Project File Map") omits numerous production files currently present in the repository:
  * Backend: `src-tauri/src/p2p/anchor_voucher.rs`, `src-tauri/src/p2p/bandwidth.rs`, `src-tauri/src/p2p/network_profile.rs`, `src-tauri/src/p2p/rendezvous.rs`, `src-tauri/src/p2p/sync.rs`, `src-tauri/src/storage/hosting_worker.rs`, `src-tauri/src/storage/path_utils.rs`, `src-tauri/src/storage/pruning.rs`, `src-tauri/src/storage/scarcity.rs`, `src-tauri/src/commands/backup_cmds.rs`, and `src-tauri/src/commands/diagnostic_cmds.rs`.
  * Frontend: `src/components/ImportingBanner.tsx`, `src/components/OfflineBanner.tsx`, `src/components/SoleSeedBanner.tsx`, `src/components/Explorer/UploadQueueDrawer.tsx`, `src/components/Explorer/ZipImportModal.tsx`, `src/components/Onboarding/` (`ConnectionDoctor.tsx`, `CredentialsStep.tsx`, `OnboardingAdvanced.tsx`, `OnboardingSimple.tsx`, `OnboardingWizard.tsx`), `src/components/Settings/BackupSettings.tsx`, and `src/components/Settings/NetworkSettings.tsx`.

## DEV_FOUND_ISSUES.md

* **Missing Resolution Logs for Versions 0.1.24 and 0.1.25:** The document only logs issues up to Version 0.1.23. Architectural fixes introduced in Version 0.1.24 (Single-Store Architecture and Redb v3 lock resolution) and Version 0.1.25 (14 security audit vulnerability remediations) are not logged.

## SECURITY_AUDIT_REPORT.md

* **Stale Overall Posture and Test Verification Results:** The Executive Summary lists overall posture as "HIGH RISK" and notes `cargo clippy --workspace -- -D warnings` as `FAILED (14 errors)`. Following the Step 53 remediation patch, all 14 vulnerabilities (VULN-01 to VULN-14) have been resolved, `cargo clippy --workspace -- -D warnings` passes with zero errors/warnings, and all 49 workspace tests pass. The report document does not yet reflect the resolved remediation status.


---

# Git Commit History Audit: Claims vs. Reality

**Audit Scope:** Git commit history (all 79 commits), analyzing commit messages against actual file diffs (`git show`) to detect unfulfilled claims, undocumented changes, or discrepancies.

---

## 1. Non-Descriptive or Misleading Commit Messages

### `859cf67` — `"e"`
* **Claim:** Minimal placeholder message (`"e"`).
* **Reality:** Massive initial repository commit containing **57 files and 9,350 lines of code**. Introduced the entire application skeleton, including Tauri v2 configuration, Iroh P2P networking, SQLite database schema, file viewer components (PDF/Markdown), Docker infrastructure, and GitHub Actions workflows without descriptive documentation.

### `a7f9f7d` — `"building builds"`
* **Claim:** Vague build modification message (`"building builds"`).
* **Reality:** Permanently deleted the standalone CI test workflow `.github/workflows/test.yml` (49 lines) and expanded `.github/workflows/build.yml` with release packaging steps. Caused permanent documentation drift with `AGENTS.md` (which still mandates `test.yml`).

---

## 2. Unfulfilled or Partially Implemented Commit Claims

### `a843d3d` — `"fix: resolve spec audit gaps - auto-updater, folder export, backup restore, unrecognized network prompt & anchor vouchers (v0.1.7)"`
* **Claimed:** Implementation of an in-app "auto-updater".
* **Reality (Partial Implementation / Mock):** The commit only wired a dummy background event in `src-tauri/src/lib.rs` emitting a hardcoded string `version: "0.2.0"` on startup. It did not query GitHub releases, did not download installer binaries, and did not execute upgrades. Real update verification and execution was not implemented until commit `4cd509b` (v0.1.12), `28e8d86` (v0.1.17), and `9ccd42b` (v0.1.21).

### `d924f50` — `"feat: proper relay server, rate limiter unit tests, live workspace file queries, and .cargo target configuration"`
* **Claimed:** "proper relay server, rate limiter unit tests...".
* **Reality:** Unit tests for `SessionLimiter` were implemented in `src-tauri/src/bin/anchor.rs:241-275`, but the rate limiter was never integrated into the relay or blob transfer protocols. The limiter remains completely isolated from network I/O.

### `14971d9` — `"release: v0.1.0-alpha.1 with Phase 2, security audit hardening, UI polish, and event-driven network monitoring"`
* **Claimed:** Release `v0.1.0-alpha.1`.
* **Reality:** Monolithic commit spanning **39 files and 5,188 lines of code** implementing almost the entirety of Phase 2 (R14 through R24). However, package versions in `package.json`, `Cargo.toml`, and `tauri.conf.json` were kept at `0.1.0` without any `-alpha.1` prerelease tag.

---

## 3. Undocumented Reversals and Side-Effects

### `74d0ff6` — `"fix(ci): resolve npm ci peer dependency conflicts, fix pdfjs canvas render and jest-dom matchers, bump to 0.1.11"`
* **Claimed:** Fixing CI peer dependency conflicts and tests.
* **Reality (Undocumented Major Rollbacks):** The commit quietly reverted 6 automated Dependabot PR merges that had silently broken the build:
  * Downgraded `tailwindcss` from `4.3.3` back to `3.4.17`.
  * Downgraded `react-dom` from `19.3.1` back to `18.3.1`.
  * Downgraded `pdfjs-dist` from `6.5.438` back to `4.9.155`.
  * Downgraded `vite` from `8.3.6` back to `6.4.5`.
  * Downgraded `jsdom` from `30.1.3` back to `25.0.1`.
  * Downgraded `vitest` from `5.1.0` back to `4.1.12`.
  These rollbacks were necessary to restore stability, but were omitted from the commit description.

### `742b518` — `"fix(security): update iroh to 1.4, patch vulnerabilities, fix zip import NotReadableError, bump to 0.1.10"`
* **Claimed:** Iroh upgrade and zip import fixes.
* **Reality (Undocumented File Deletion):** In addition to the claimed fixes, the commit deleted `src-tauri/Cargo.lock` (10,460 lines removed), leaving the Tauri crate build unpinned until it was restored in `4cd509b`.

---

## 4. Accurate and High-Fidelity Commits

The following recent commits accurately and faithfully implemented what their commit messages and scopes claimed:

* **`e22a8c2` (v0.1.25):** Addressed all 14 security findings (VULN-01 to VULN-14) from `SECURITY_AUDIT_REPORT.md`, fixed 14 clippy compiler warnings with `-D warnings`, and bumped version to `0.1.25` across all three manifests.
* **`fabb280` (v0.1.24):** Unified `BlobStore` loading in `AppState` to prevent concurrent `redb` open lock contention, added readiness polling for engine commands, and bumped version to `0.1.24`.
* **`e7145d1` (v0.1.23):** Caught `redb v2` migration errors and backed up legacy index, terminated locking processes before silent in-place updater execution, and bumped version to `0.1.23`.
* **`5082dbd` (v0.1.22):** Implemented adaptive rendezvous discovery intervals (15s searching vs 60s connected) and added readiness polling to `trigger_swarm_discovery`.
* **`9ccd42b` (v0.1.21):** Replaced trojan diagnostic port 12345 with port 53, disabled UPnP/NAT-PMP portmapper, added university campus SSID auto-detection, and added a 1-click in-app update button.
* **`ffd7251` (v0.1.20):** Added HTML redirect fallback to bypass GitHub API rate limits (HTTP 403) on shared campus IPs.
* **`39bbfac` (v0.1.19):** Implemented automated HTTPS rendezvous discovery (`src-tauri/src/p2p/rendezvous.rs`) over port 443.
* **`ca91d53` (v0.1.18):** Fixed IPC parameter naming mismatch (`nodeAddrOrTicket` vs `node_id`) in `connect_peer`.
* **`28e8d86` (v0.1.17):** Configured silent `/S` in-place NSIS upgrades without uninstall dialogs.
* **`eb565c5` (v0.1.16):** Added preloaded `BlobStore` on boot, disk NodeId identity fallback, and direct offline cache reading.
