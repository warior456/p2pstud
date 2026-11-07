# Goldfish Annotated Review Report

Status: resolved
Target: `2026-11-07_DRIFT_REPORT.md` (`full file`)

## Prioritized Findings & AI Suggestions

### [Issue 12] [`egm-security`] - Severity: Blocker
*   **Affected Scope:** [Lines 52-56](SECURITY_AUDIT_REPORT.md#L1-L40) (Executive Summary & Posture)
*   **Discrepancy Description:** The Executive Summary in `SECURITY_AUDIT_REPORT.md` lists overall security posture as "HIGH RISK" and notes `cargo clippy --workspace -- -D warnings` as `FAILED (14 errors)`. Following the Step 53/v0.1.25 remediation patch, all 14 vulnerabilities (VULN-01 to VULN-14) were resolved, clippy passes with zero warnings, and all 49 workspace tests pass.
*   **Rationale:** Retaining a stale "HIGH RISK" marker and failed compiler check contradicts the current verified repository state and confuses auditors.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** `SECURITY_AUDIT_REPORT.md` should accurately document both the historical findings and the verified remediation status achieved in version 0.1.25.
*   ***Proposed Edit:***
    *   Location: `SECURITY_AUDIT_REPORT.md` (Lines 1-40)
    *   Change: Update Overall Risk Rating to "RESOLVED / SECURED (v0.1.25)", update compiler audit status to "PASSED (0 errors, 0 warnings)", and prepend a Remediation Verification Banner.

---

### [Issue 1] [`egm-comprehension`] - Severity: Major
*   **Affected Scope:** [Lines 8-15](docs/KULEUVEN_AUTH_SPEC.md#L1-L172) (Authentication & Ticket Architecture)
*   **Discrepancy Description:** `GroepTMemberTicket` data structures, QUIC mutual ticket verification checklist, backend email OTP issuance endpoint (`POST /api/auth/verify-otp`), Microsoft Entra ID integration, and Swarm CRL broadcasting are absent in `src-tauri/src/`.
*   **Rationale:** The specification defines a complete student ticket validation flow, but the codebase authenticates QUIC connections solely via passphrase-derived dynamic ALPN.
*   ***AI Suggestion:*** ESCALATE
*   ***Justification:*** As noted in `docs/KULEUVEN_AUTH_SPEC.md:3`, this document is explicitly marked `> **Status:** Proposed / Design Specification (Not Implemented)`. The user has explicitly stated: *"the otp flow or similar should not be implemented yet but do make a proper plan or updated suggestions for that (i don't have a domain and only a very light server available)"*. No code changes are to be made to `src-tauri/` at this time. Instead, the design specification should be updated with an ultra-lightweight, zero-domain architecture plan tailored to a low-spec VPS (e.g. 512MB RAM) and zero custom domain requirements.
*   ***Proposed Edit:***
    *   Location: `docs/KULEUVEN_AUTH_SPEC.md` (Sections 1, 3, 6)
    *   Change: Reaffirm status as Deferred / Phase 3 Specification. Replace heavy server components (Redis, multi-worker external services, heavy Argon2id memory buffers) with an Ultra-Lightweight Zero-Domain Architecture plan:
        1. Outbound delivery via authenticated Gmail SMTP (`smtp.gmail.com:587` with App Password) requiring 0 domains and passing KU Leuven Microsoft 365 Defender DKIM/SPF checks.
        2. In-memory ephemeral OTP state in the Rust anchor process (`< 100 KB` RAM) with 10-minute sliding TTL and token-bucket rate limiting, requiring 0 external databases.
        3. Tuned memory-safe cryptographic hashing (Argon2id m=16MB or BLAKE3 + campus pepper) to eliminate OOM risks on light servers.

---

### [Issue 2] [`egm-codebase-details`] - Severity: Major
*   **Affected Scope:** [Lines 17-18](README.md#L18-L25) (Anchor Egress Rate Limiting)
*   **Discrepancy Description:** `README.md` claims active egress throttling (150 KB/s per session, 2 MB/s aggregate). While `SessionLimiter` is implemented and unit-tested in `src-tauri/src/bin/anchor.rs:184-239`, it is not wired into the QUIC router or `BlobsProtocol` transfer stream.
*   **Rationale:** Users and operators deploying the anchor node expect hard egress throttling to protect cloud VPS bandwidth, but network transfers currently bypass the limiter.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Documentation in `README.md` and `REQUIREMENTS.md` must accurately reflect that `SessionLimiter` is an implemented token-bucket algorithm staged for custom Iroh stream wrapper integration, rather than an actively intercepting proxy.
*   ***Proposed Edit:***
    *   Location: `README.md` ("Headless Anchor Node Deployment" & "Key Features")
    *   Change: Update feature description to clarify that token-bucket egress rate limiting is staged in `anchor.rs` and document instructions for OS-level egress limiting (`tc` / `wondershaper`) on VPS deployments until Iroh 1.4 stream wrapper hooks are attached.

---

### [Issue 3] [`egm-codebase-details`] - Severity: Major
*   **Affected Scope:** [Lines 19-20](README.md#L140-L155) (Anchor `--workspace-id` Parameter)
*   **Discrepancy Description:** In `src-tauri/src/bin/anchor.rs:49`, `workspace_id` is defined as `Option<String>` and is never referenced or validated, despite `README.md` and `REQUIREMENTS.md` specifying it as a required parameter.
*   **Rationale:** Command-line operators are instructed to supply a flag that has no operational effect.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Align documentation and CLI description to clearly define `--workspace-id` as an optional workspace namespace filter reserved for multi-tenant anchor deployments.
*   ***Proposed Edit:***
    *   Location: `README.md` & `src-tauri/src/bin/anchor.rs`
    *   Change: Clarify in CLI help and `README.md` that `--workspace-id` is optional; when omitted, the anchor serves the default shared swarm catalog.

---

### [Issue 4] [`egm-structural`] - Severity: Major
*   **Affected Scope:** [Lines 20-21](README.md#L10-L15) & [AGENTS.md#L100-L120](AGENTS.md#L100-L120) (Frontend Framework Ambiguity)
*   **Discrepancy Description:** Documentation references Svelte alongside React, whereas the implementation is strictly React 18 with Vite and TypeScript (`src/App.tsx`).
*   **Rationale:** Creates confusion for frontend contributors and subagents regarding component technology and tooling.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Remove all legacy references to Svelte across `README.md` and `AGENTS.md`.
*   ***Proposed Edit:***
    *   Location: `README.md` and `AGENTS.md`
    *   Change: Replace "Svelte/React" with "React 18 (Vite, TypeScript, Tailwind CSS)".

---

### [Issue 5] [`egm-codebase-details`] - Severity: Major
*   **Affected Scope:** [Lines 23-24](REQUIREMENTS.md#L120-L240) (Outdated Status for Requirements R14 through R24)
*   **Discrepancy Description:** Requirements R14.1 through R24.2 (Bilateral Seeding Receipts, Scarcity Replication, Sole Seed Warnings, Stealth Mode, File Immutability, Course Folders, Ratings & Quarantine, Onboarding & Connection Doctor, Transfer Resume Queue, Backup/ZIP Export, and Lifecycle Pruning) are marked as pending (`⬜`), despite full implementation in `src-tauri/` and `src/`.
*   **Rationale:** Disconnects product requirements tracking from the actual state of the application.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Update all implemented requirement checkboxes in `REQUIREMENTS.md` from `⬜` to `✅`.
*   ***Proposed Edit:***
    *   Location: `REQUIREMENTS.md` (Sections R14 to R24)
    *   Change: Change pending checkboxes (`⬜`) to completed (`✅`) and reference corresponding modules (`anchor_voucher.rs`, `bandwidth.rs`, `network_profile.rs`, `rendezvous.rs`, etc.).

---

### [Issue 6] [`egm-codebase-details`] - Severity: Major
*   **Affected Scope:** [Lines 25-31](REQUIREMENTS.md#L75-L105) (R7 Dependency Matrix Drift)
*   **Discrepancy Description:** R7 lists outdated library versions (`iroh = 0.29`, `iroh-blobs = 0.29`, `dirs = 5`, `governor = 0.6`) and omits active dependencies present in `src-tauri/Cargo.toml` (`iroh-tickets`, `zip`, `chrono`, `url`, `semver`, `reqwest`, `thiserror`, `rfd`, `tauri-plugin-autostart`).
*   **Rationale:** Misleads developers evaluating dependency compatibility and security audits.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Synchronize the R7 table in `REQUIREMENTS.md` with `src-tauri/Cargo.toml`.
*   ***Proposed Edit:***
    *   Location: `REQUIREMENTS.md` (Section R7)
    *   Change: Update versions to `iroh = 1.4`, `iroh-blobs = 0.104`, `dirs = 7`, `governor = 0.7`, and append the missing dependencies.

---

### [Issue 7] [`egm-structural`] - Severity: Major
*   **Affected Scope:** [Lines 37-38](AGENTS.md#L130-L145) (Docker File Ownership)
*   **Discrepancy Description:** `AGENTS.md` assigns AnchorDaemonAgent write ownership over `docker/Dockerfile.anchor`, `docker/Dockerfile.node`, and `docker/docker-compose.single.yml`. These files were consolidated into `docker/Dockerfile`, `docker/docker-compose.yml`, and `docker/p2pstud-anchor.service`.
*   **Rationale:** Subagents attempt to look for or modify nonexistent files.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Update the file ownership manifest in `AGENTS.md`.
*   ***Proposed Edit:***
    *   Location: `AGENTS.md` (AnchorDaemonAgent section)
    *   Change: Update file list to `docker/Dockerfile`, `docker/docker-compose.yml`, and `docker/p2pstud-anchor.service`.

---

### [Issue 8] [`egm-structural`] - Severity: Nit
*   **Affected Scope:** [Lines 39-40](AGENTS.md#L165-L175) (Unchecked Test Checklist)
*   **Discrepancy Description:** AGENTS.md Section 5 displays unchecked boxes (`[ ]`) for unit tests that are all implemented and passing in `src-tauri/`.
*   **Rationale:** Gives a false impression that initial unit tests remain unwritten.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Check the boxes to reflect passing status.
*   ***Proposed Edit:***
    *   Location: `AGENTS.md` (Section 5)
    *   Change: Replace `[ ]` with `[x]` for all 8 listed test suites.

---

### [Issue 9] [`egm-structural`] - Severity: Major
*   **Affected Scope:** [Lines 42-44](AGENT_PLAN.md#L45-L70) (Progress Tracker Row Omission)
*   **Discrepancy Description:** The header of `AGENT_PLAN.md` states that Step 53 completed addressing all 14 security findings from `SECURITY_AUDIT_REPORT.md` and bumped the version to `0.1.25`. However, row 53 in the Progress Tracker records "Single-Store Architecture ... bumped version to 0.1.24", omitting a dedicated row for the security remediations.
*   **Rationale:** Breaks sequence traceability in the progress tracking log.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Align table entries: preserve row 53 as Single-Store Architecture (v0.1.24) and add row 54 for Security Audit Remediations & Clippy Zero-Warnings (v0.1.25).
*   ***Proposed Edit:***
    *   Location: `AGENT_PLAN.md` (Progress Tracker table)
    *   Change: Add Step 54 entry detailing the v0.1.25 security remediations and update current status header.

---

### [Issue 10] [`egm-structural`] - Severity: Major
*   **Affected Scope:** [Lines 45-47](AGENT_PLAN.md#L75-L120) (Incomplete Project File Map)
*   **Discrepancy Description:** Section 3 of `AGENT_PLAN.md` omits 11 backend files and 9 frontend components created during Phase 2.
*   **Rationale:** Developers relying on the file map are unaware of key modules like `rendezvous.rs`, `bandwidth.rs`, `anchor_voucher.rs`, etc.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Update the file tree in `AGENT_PLAN.md` with all current repository source files.
*   ***Proposed Edit:***
    *   Location: `AGENT_PLAN.md` (Section 3: Project File Map)
    *   Change: Insert missing backend and frontend file paths into the directory hierarchy tree.

---

### [Issue 11] [`egm-structural`] - Severity: Major
*   **Affected Scope:** [Lines 49-51](DEV_FOUND_ISSUES.md#L150-L175) (Missing Resolution Logs for v0.1.24 and v0.1.25)
*   **Discrepancy Description:** `DEV_FOUND_ISSUES.md` ends at Version 0.1.23, lacking entries for v0.1.24 (Single-Store Redb lock resolution) and v0.1.25 (14 security audit vulnerability remediations).
*   **Rationale:** Incomplete historical issue log.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Append entries for v0.1.24 and v0.1.25 into `DEV_FOUND_ISSUES.md`.
*   ***Proposed Edit:***
    *   Location: `DEV_FOUND_ISSUES.md`
    *   Change: Add comprehensive entries documenting the issues found, root causes, and resolutions for v0.1.24 and v0.1.25.

---

### [Issue 13] [`egm-codebase-details`] - Severity: Nit
*   **Affected Scope:** [Lines 59-125](2026-11-07_DRIFT_REPORT.md#L59-L125) (Git Commit History Audit)
*   **Discrepancy Description:** Historical git commit messages contain vague descriptions (e.g. `859cf67 "e"`), unfulfilled claims (e.g. `d924f50` rate limiter not wired), and undocumented rollbacks (e.g. `74d0ff6` Dependabot downgrades).
*   **Rationale:** Historical git commits are recorded in the repository object database.
*   ***AI Suggestion:*** CONTEST
*   ***Justification:*** Git commit history is immutable. Force-pushing or rewriting published history (`git rebase -i` or `git filter-repo`) would invalidate all downstream clones and commit hashes. The underlying technical and documentation discrepancies caused by these commits are being directly resolved in the documentation and codebase via the proposals above.
*   ***Proposed Edit:***
    *   Location: None (Decline rewriting commit history; record findings in documentation).
    *   Change: None.
