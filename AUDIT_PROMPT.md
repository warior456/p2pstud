# Comprehensive Security, Architecture & Protocol Audit Prompt
## Project: `p2pstud` — P2P Cooperative Campus Workspace

> **Purpose:** This document provides the complete, authoritative audit prompt and methodology for conducting an exhaustive, adversarial, and architectural security audit of the `p2pstud` codebase.
> It is designed to be executed by senior security researchers, red teams, or advanced autonomous reasoning models (e.g., Claude 3.7 Sonnet / Opus, GPT-4.5 / o3, Gemini 1.5 Pro).
>
> **Scope:** Full-stack audit across Rust backend (`src-tauri/`), Tauri v2 host boundary (`tauri.conf.json`, IPC commands), React frontend (`src/`), embedded Iroh P2P protocol, SQLite/Redb storage engines, campus stealth/IDS evasion, auto-updater supply chain, and the proposed KU Leuven ticket authentication specification (`docs/KULEUVEN_AUTH_SPEC.md`).

---

# Part 1: The Master Audit Prompt (All-in-One)

*Copy and paste the entire prompt block below into your auditing environment or AI reasoning assistant.*

```markdown
# MISSION BRIEF: EXHAUSTIVE SECURITY & ARCHITECTURAL AUDIT OF `p2pstud`

You are acting as an elite Principal Security Researcher, Distributed Systems Architect, and Cryptographic Protocol Auditor. Your objective is to perform an exhaustive, uncompromising security, reliability, and architectural audit of the `p2pstud` project.

## 1. Project Context & Threat Environment
`p2pstud` is a cross-platform (Windows & Linux) academic peer-to-peer file-sharing application designed to operate over hostile, restrictive enterprise university networks (specifically **Eduroam**, **KU Leuven / Groep T**, **KotNet**, and campus Wi-Fi with client isolation and symmetric NAT).
It embeds **Iroh** (QUIC transport, BLAKE3 content-addressed blob storage, TLS 443 relay fallback), **Tauri v2** (Rust host shell + React/Vite WebView), **SQLite** (rusqlite with WAL mode for file metadata and consensus state), and a headless cloud **Anchor VPS binary** (`src/bin/anchor.rs`).

### Crucial Historical & Operational Realities:
1. **University IDS Triggers (Prior Incident v0.1.21):** The project previously triggered an enterprise Intrusion Detection System (IDS / Palo Alto / Suricata) alert on campus networks (`Type: malware: BitTorrent distribution and connections to malware IPs`). The root causes were an outbound diagnostic UDP probe to port `12345` (the NetBus trojan port) and Iroh's default UPnP SSDP multicast packets (`239.255.255.250:1900`) and NAT-PMP broadcasts (`224.0.0.1:5351`), which firewalls classified as unauthorized P2P/BitTorrent activity.
2. **Campus Wi-Fi Isolation:** Eduroam drops peer-to-peer UDP broadcasts (mDNS, SSDP) and symmetric NAT prevents direct UDP hole punching, requiring automated HTTPS rendezvous (`ntfy.sh` over port 443) and encrypted QUIC relay fallbacks over TLS port 443 (`iroh-relay`).
3. **Decentralized Authentication Spec (`docs/KULEUVEN_AUTH_SPEC.md`):** A proposed capability ticket authentication system using Ed25519-signed tickets issued via institutional student email OTP (`@student.kuleuven.be`) to eliminate the "Discord/WhatsApp leak" vulnerability of shared network passphrases.
4. **Host-to-Play Cooperative Cache:** Nodes share unpinned course materials up to an 80% quota target, with automatic LRU eviction at 90%, dwell time guards, sole-seed protection, and 86% quorum-based community deletion voting.
5. **In-Place Auto-Updating:** A detached PowerShell execution pipeline that downloads release assets from GitHub and silently invokes NSIS with `/S` while terminating the running process.

---

## 2. Core Audit Methodology & Required Tracks

Perform a line-by-line inspection of the codebase across the following **8 Mandatory Audit Tracks**:

### TRACK 1: P2P Network Protocol, Campus Stealth & Anti-IDS Evasion
- **Files:** `src-tauri/src/p2p/engine.rs`, `src-tauri/src/p2p/network_profile.rs`, `src-tauri/src/p2p/relay.rs`, `src-tauri/src/p2p/rendezvous.rs`, `src-tauri/src/commands/diagnostic_cmds.rs`
- **Audit Focus:**
  1. **SSDP/UPnP & NAT-PMP Elimination:** Verify whether any code path (including Iroh defaults, third-party crates, or test harnesses) can initialize portmapping or multicast discovery on campus networks.
  2. **Port Probing & Diagnostic Safety:** Audit `diagnostic_cmds.rs` and network connectivity checks. Verify that no probes target non-standard ports, suspicious ranges, or arbitrary external IPs that could trigger IDS/IPS heuristics.
  3. **ALPN Swarm Isolation:** Audit how `get_sync_alpn(&db)` is derived. Can a malformed or empty passphrase cause ALPN collision? Does an open network expose user catalog metadata to arbitrary peers on the same relay?
  4. **HTTPS Rendezvous Discovery (`rendezvous.rs`):**
     - Audit privacy exposure to `ntfy.sh` (or third-party rendezvous servers). Are `NodeId`, IP addresses, or ticket tokens exposed in cleartext?
     - **Unauthenticated Announcement Spoofing:** Can an adversary publish forged tickets on the topic?
     - **Denial of Service / Dial Amplification:** Can an attacker flood a topic with 10,000 bogus tickets to force legitimate student clients into a connection storm, exhausting sockets, memory, or CPU?
  5. **Campus Profile Detection:** Verify SSID matching regexes (`kuleuven`, `campus`, `eduroam`, `kotnet`, `student`, `associatie`). Can an attacker bypass detection using Unicode lookalikes or hidden SSIDs?

### TRACK 2: Cryptographic Primitives, Key Management & Authentication Spec
- **Files:** `src-tauri/src/p2p/identity.rs`, `src-tauri/src/p2p/anchor_voucher.rs`, `src-tauri/src/p2p/sync.rs`, `docs/KULEUVEN_AUTH_SPEC.md`
- **Audit Focus:**
  1. **SecretKey Storage & ACLs:** Audit `load_or_create_secret_key`. Are file permissions strictly restricted to the user (`0o600` on Unix, restrictive DACL on Windows)? Are atomic write guarantees (`tempfile` + `rename`) race-free?
  2. **Content Addressing & Verification:** Verify that every blob or chunk retrieved from a remote peer is verified against its BLAKE3 hash before being committed to the store or served.
  3. **Voucher & Receipt Signatures:** Audit `anchor_voucher.rs` and `test_seeding_receipt_signing_and_verification`. Are nonces, timestamps, and signatures immune to replay, truncation, or malleability attacks?
  4. **Deep Audit of `KULEUVEN_AUTH_SPEC.md`:**
     - **Email Salted Hash:** Is `BLAKE3(salt + student_email)` sufficient against pre-computation dictionary attacks given that student numbers (`r0123456`) have low entropy (~$10^7$ combinations)?
     - **Ticket Replay & Clock Skew:** Does ticket validation tolerate clock skew securely? Can expired tickets be resurrected?
     - **NodeId Binding:** Does the handshake strictly prove possession of the private key corresponding to `ticket.node_id` via a cryptographic challenge-response?

### TRACK 3: Host Boundary, Tauri v2 IPC & Webview Sandboxing
- **Files:** `src-tauri/src/lib.rs`, `src-tauri/src/commands/*.rs`, `src-tauri/tauri.conf.json`, `src/App.tsx`, `src/components/Viewers/*`
- **Audit Focus:**
  1. **Tauri v2 Scoping & Capabilities:** Inspect `tauri.conf.json`. Are dangerous plugins (`tauri-plugin-shell`, `tauri-plugin-fs`) over-privileged? Can arbitrary shell commands be executed from the WebView?
  2. **Information Disclosure in IPC:** Audit all error mappings (`.map_err(|e| e.to_string())`). Do errors leak full system paths, username folders, or internal database schemas into the JavaScript frontend?
  3. **XSS & Preview Sanitization:**
     - Audit Markdown rendering (`MarkdownEditor.tsx` / `marked` + `DOMPurify`). Is `DOMPurify` configured strictly? Are custom URL schemes (`javascript:`, `data:`, `file:`, `vbscript:`) completely sanitized?
     - Audit PDF viewer (`PdfViewer.tsx`). Is embedded JavaScript in malicious PDF files disabled?
  4. **Single-Instance Mutex & Process Lifecycle:** Verify `tauri-plugin-single-instance` handling and system tray close prevention (`WindowEvent::CloseRequested`). Can an attacker cause a WebView2 deadlock or zombie process?

### TRACK 4: Storage Engine, Concurrency & Redb / SQLite Integrity
- **Files:** `src-tauri/src/storage/db.rs`, `src-tauri/src/storage/blob_tracker.rs`, `src-tauri/src/storage/eviction.rs`, `src-tauri/src/storage/hosting_worker.rs`, `src-tauri/src/storage/scarcity.rs`
- **Audit Focus:**
  1. **Eviction Loop Logic:** Audit `eviction.rs`.
     - *Specific Bug to Investigate:* Does the `while used > low_mark` loop contain an unconditional `break` statement that prevents complete eviction to the low watermark?
     - Check priority order: does LRU evict abundant files ($\ge 3$ seeds) before rare files?
     - Does the dwell time guard (24-48h) or sole-seed protection create deadlock conditions where disk space cannot be freed even when quota is exceeded?
  2. **SQLite Concurrency & Multi-Network Isolation:**
     - Audit `db.rs` lock acquisition (`db.lock()`). Are long transactions holding the mutex across async await points?
     - Verify database switching (`switch_network_database`). Are open statements finalized? Is cross-network data leakage impossible?
  3. **Redb BlobStore Concurrency:** Verify `load_store` vs `IrohEngine::start`. Can multiple threads or processes trigger a lock conflict on `blobs.db`? Is the `blobs.db.v2.bak` fallback safe against database corruption?

### TRACK 5: Filesystem Sanitization, Path Traversal & Zip-Slip
- **Files:** `src-tauri/src/storage/path_utils.rs`, `src-tauri/src/commands/file_cmds.rs`, `src-tauri/src/commands/backup_cmds.rs`
- **Audit Focus:**
  1. **Path Sanitization (`safe_join` & `safe_join_new`):**
     - Audit against Windows-specific path bypasses: UNC paths (`\\?\`, `\\.\`), alternate data streams (`::$DATA`), trailing dots and spaces (`file.txt. `), and reserved DOS device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`).
     - Audit against null-byte injection (`\0`) and Unicode normalization exploits (NFC vs NFD equivalence).
     - Audit `sanitize_virtual_path`: can double slashes or directory traversal bypass virtual folder structures?
  2. **Zip-Slip & Multi-GB ZIP Streaming:**
     - Audit `import_zip_file` and `import_zip_bytes`. Does extraction strictly sanitize `entry.name()` before writing?
     - **Zip Bomb Defense:** Is decompression ratio or maximum extracted byte size enforced, or can a 10 MB archive unpack into 500 GB, crashing the node?
     - Are symlinks inside ZIP archives rejected or followed outside the cache root?

### TRACK 6: Distributed Consensus, Governance & Anti-Abuse
- **Files:** `src-tauri/src/storage/blob_tracker.rs`, `src-tauri/src/commands/file_cmds.rs`, `src-tauri/src/p2p/sync.rs`
- **Audit Focus:**
  1. **Community Delete & Keep Voting:**
     - Audit the 86% consensus threshold and 3-vote quorum.
     - **Sybil Voting Attack:** Can a rogue student spawn multiple local `NodeId`s to unilaterally reach the 3-vote quorum and 86% threshold to delete valid lecture materials?
     - **Keep Defense:** Can an attacker cancel a legitimate delete vote by voting Keep with fake identities?
  2. **Emergency Quarantine Abuse:**
     - 5 reports trigger an immediate 1-week quarantine hold. Can 5 colluding peers censor exam notes on the eve of an exam? Is there rate-limiting or reputation attached to reports?
  3. **File Move / Rename Authorization:** Verify that only the verifiable creator/owner (`owner_node_id`) can move or rename files.

### TRACK 7: Auto-Updater, Detached Execution & Supply Chain
- **Files:** `src-tauri/src/commands/backup_cmds.rs`, `src-tauri/Cargo.toml`, `.github/workflows/build.yml`
- **Audit Focus:**
  1. **Installer Verification (CRITICAL):**
     - Audit `download_app_update` and `install_app_update`.
     - **Is there cryptographic signature or SHA-256 hash verification of the downloaded installer before execution?**
     - If a compromised GitHub account or an attacker-controlled network redirects `releases/latest`, can an arbitrary malicious executable be silently executed with `/S`?
  2. **Detached Script Command Injection:**
     - Audit the generated PowerShell script in `install_app_update`.
     - Is `installer_path` or `current_exe` vulnerable to PowerShell command injection if paths contain single quotes, semicolons, or format strings?
  3. **Dependency Vulnerability & Supply Chain:** Inspect `Cargo.toml` and `Cargo.lock` for unpinned or vulnerable dependencies (e.g., past CVEs in `zip` or transitive crates).

### TRACK 8: Async Concurrency, Deadlocks & Resource Exhaustion (DoS)
- **Files:** `src-tauri/src/lib.rs`, `src-tauri/src/storage/*`, `src-tauri/src/p2p/*`
- **Audit Focus:**
  1. **Tokio Runtime Starvation:** Are blocking I/O calls (`std::fs`, SQLite synchronous queries, ZIP compression) executed inside async functions without `tokio::task::spawn_blocking`?
  2. **Deadlock Analysis:** Audit lock hierarchies between `tokio::sync::Mutex`, `tokio::sync::RwLock`, and `std::sync::Mutex<rusqlite::Connection>`. Can a deadlock occur between `AppState` guards and background worker loops?
  3. **Memory Budget & Leaks:** Verify that large file streaming does not buffer entire multi-gigabyte files into RAM (strictly enforce the ≤ 120 MB idle RAM constraint).
  4. **Background Worker Crash Resilience:** What happens if `eviction_worker`, `scarcity_worker`, or `rendezvous` encounters an unhandled error? Does it crash the entire node or fail silently without retrying?

---

## 3. Required Deliverable Format

Structure your findings using the following strict schema for each vulnerability or architectural defect discovered:

```markdown
### [VULN-ID] Short Descriptive Title
- **Severity:** [Critical | High | Medium | Low | Informational]
- **CVSS v3.1 / v4.0 Score & Vector:** (e.g., CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H - 9.8)
- **Affected Component(s):** Exact file paths and line ranges (e.g., `src-tauri/src/commands/backup_cmds.rs#L649-L724`)
- **Vulnerability Category:** (e.g., Supply Chain Execution, Path Traversal, DoS, Cryptographic Flaw, Concurrency Deadlock)

#### 1. Vulnerability Description & Technical Root Cause
Detailed explanation of why the code is vulnerable, including protocol flows, function calls, and architectural oversights.

#### 2. Exploit Scenario / Proof of Concept (PoC)
Concrete, reproducible step-by-step attack scenario showing how an adversary (or malicious peer/network) triggers the vulnerability. Include exploit scripts, crafted payloads, or CLI test cases where applicable.

#### 3. Impact Assessment
Real-world consequences on students, campus infrastructure, data integrity, privacy, or device security.

#### 4. Remediation & Code Diff
Exact code patch showing the fix:
```diff
- // Vulnerable code
+ // Secure replacement
```
```

---

## 4. Pre-Audit Automated Verification Checklist
Before starting manual review, execute the following commands in the workspace and integrate the findings into your analysis:
1. `cargo clippy --workspace -- -D warnings` — check for compiler and clippy warnings.
2. `cargo test --workspace` — verify baseline unit and integration test health.
3. `cargo audit` (if cargo-audit is installed) — scan dependency tree for known CVEs.
4. `npm run test` — run frontend Vitest component tests.
5. `npm run build` — verify TypeScript types and production bundle compilation.

Begin your audit now. Be thorough, skeptical, adversarial, and mathematically precise.
```

---

# Part 2: Modular Deep-Dive Sub-Prompts

*When conducting targeted reviews with specialized agents or within constrained context windows, use these domain-specific sub-prompts.*

---

## Sub-Prompt A: Campus Stealth, Network Protocol & Anti-IDS Evasion Deep Dive

```markdown
# AUDIT SUB-PROMPT A: CAMPUS STEALTH & ANTI-IDS EVASION (`p2pstud`)

You are an expert Network Security Auditor and Forensic Protocol Analyst specializing in enterprise intrusion detection systems (IDS/IPS, Suricata, Snort, Palo Alto) and university Wi-Fi architectures (Eduroam, 802.1X, symmetric NAT, client isolation).

### Scope:
- `src-tauri/src/p2p/engine.rs`
- `src-tauri/src/p2p/network_profile.rs`
- `src-tauri/src/p2p/relay.rs`
- `src-tauri/src/p2p/rendezvous.rs`
- `src-tauri/src/commands/diagnostic_cmds.rs`

### Investigation Checklist:
1. **SSDP / UPnP Multicast & NAT-PMP Broadcasts:**
   - Audit `engine.rs` endpoint builder. Is `PortmapperConfig::Disabled` enforced across all fallback builders?
   - Does `iroh-relay` or `iroh-net` attempt any background SSDP discovery on `239.255.255.250:1900` or NAT-PMP on `224.0.0.1:5351`?
2. **Diagnostic Probe Safety (`diagnostic_cmds.rs`):**
   - Inspect every outbound network probe. Are any probes directed to non-standard or suspicious ports (e.g., past port 12345 NetBus alert)?
   - Does DNS resolution use trusted system resolvers or non-suspicious public resolvers (1.1.1.1:53)?
3. **Rendezvous Protocol Analysis (`rendezvous.rs`):**
   - Analyze the `ntfy.sh` polling and publish mechanisms.
   - Can a rogue peer forge arbitrary announcements or tickets?
   - What happens if an adversary floods the topic with 1,000 requests per minute?
   - Does the HTTPS traffic mimic normal web traffic, or can DPI (Deep Packet Inspection) detect repetitive polling signatures?
4. **Campus SSID Detection Heuristics:**
   - Review `network_profile.rs`. How is the current SSID obtained on Windows (`netsh wlan show interfaces`) and Linux (`nmcli`, `/proc/net/wireless`)?
   - Can SSID spoofing, special characters, or network disconnections cause fallback to open non-stealth profiles?

Deliver a detailed technical report with root cause analysis, PoCs, and remediation diffs.
```

---

## Sub-Prompt B: Cryptographic Architecture & Decentralized Ticket Spec Deep Dive

```markdown
# AUDIT SUB-PROMPT B: CRYPTOGRAPHIC PRIMITIVES & AUTHENTICATION SPEC (`p2pstud`)

You are a Cryptographic Engineer and Protocol Security Auditor specializing in Ed25519 signatures, BLAKE3 content addressing, capability tickets, and decentralized identity.

### Scope:
- `src-tauri/src/p2p/identity.rs`
- `src-tauri/src/p2p/anchor_voucher.rs`
- `src-tauri/src/p2p/sync.rs`
- `docs/KULEUVEN_AUTH_SPEC.md`

### Investigation Checklist:
1. **SecretKey Generation & Storage (`identity.rs`):**
   - Inspect `load_or_create_secret_key`. Is the key generated using a cryptographically secure RNG?
   - How are Windows ACLs applied (`icacls` vs Rust native security descriptors)? Are race conditions possible between file creation and ACL assignment?
2. **BLAKE3 Content Addressing Integrity:**
   - Trace blob download in `sync.rs` and `file_cmds.rs`. Is chunk integrity verified stream-by-stream, or can a corrupted chunk be stored in redb before validation?
3. **Audit of `docs/KULEUVEN_AUTH_SPEC.md`:**
   - **Ticket Structure:** Review `GroepTMemberTicket` fields (`node_id`, `email_hash`, `network`, `issued_at`, `expires_at`, `signature_hex`).
   - **Privacy of `email_hash`:** `BLAKE3(salt + email)`. Given that university student numbers follow predictable formats (e.g. `r` followed by 7 digits), can an attacker precompute rainbow tables for all 60,000 KU Leuven students to deanonymize all tickets? How should salt distribution and rotation be hardened?
   - **Replay & Sybil Resistance:** Can a student copy an issued ticket to multiple devices? Does the handshake prove ownership of the private key for `node_id`?
   - **Revocation:** How does the anchor authority revoke compromised tickets or abusive students?

Deliver a formal cryptographic audit with mathematical rigor and specific protocol enhancements.
```

---

## Sub-Prompt C: Tauri v2 Host Boundary, IPC & Webview Sandboxing Deep Dive

```markdown
# AUDIT SUB-PROMPT C: TAURI V2 IPC & WEBVIEW SANDBOXING (`p2pstud`)

You are an Application Security Specialist specializing in Electron/Tauri desktop architectures, WebView2/WebKitGTK security boundaries, IPC command fuzzing, and client-side sandboxing.

### Scope:
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/*.rs`
- `src/App.tsx`
- `src/components/Viewers/MarkdownEditor.tsx`
- `src/components/Viewers/PdfViewer.tsx`

### Investigation Checklist:
1. **Tauri v2 Permissions & Plugin Capabilities:**
   - Inspect `tauri.conf.json` security and bundle configurations.
   - Are `tauri-plugin-fs` and `tauri-plugin-shell` scoped strictly? Can frontend JavaScript execute arbitrary system commands or read outside the app directory?
2. **IPC Parameter Sanitization & Information Leakage:**
   - Review all `#[tauri::command]` handlers in `file_cmds.rs`, `storage_cmds.rs`, and `backup_cmds.rs`.
   - Are all inputs (strings, offsets, integers, paths) validated?
   - Does any error leak sensitive stack traces, usernames, or internal paths?
3. **DOMPurify & Markdown Preview XSS:**
   - Inspect `MarkdownEditor.tsx`. How is `marked` configured?
   - How is `DOMPurify.sanitize()` invoked? Are `FORBID_TAGS`, `FORBID_ATTR`, and allowed URI schemes (`http`, `https`) strictly configured? Can an adversary upload a markdown note containing `<svg onload=...>` or `javascript:` links that execute in WebView?
4. **PDF.js Sandboxing:**
   - Inspect `PdfViewer.tsx`. Are PDF scripts and external font loading restricted? Can a malformed PDF trigger remote code execution or freeze the renderer?

Deliver a technical vulnerability report with step-by-step reproduction and complete remediation diffs.
```

---

## Sub-Prompt D: Filesystem, Storage Engine & Zip-Slip Traversal Deep Dive

```markdown
# AUDIT SUB-PROMPT D: FILESYSTEM, STORAGE ENGINE & ZIP-SLIP TRAVERSAL (`p2pstud`)

You are a Systems Software Security Auditor specializing in filesystem security, directory traversal attacks, database concurrency (SQLite, Redb), and archive extraction vulnerabilities.

### Scope:
- `src-tauri/src/storage/path_utils.rs`
- `src-tauri/src/storage/db.rs`
- `src-tauri/src/storage/blob_tracker.rs`
- `src-tauri/src/storage/eviction.rs`
- `src-tauri/src/commands/file_cmds.rs`
- `src-tauri/src/commands/backup_cmds.rs`

### Investigation Checklist:
1. **Path Traversal Defense (`path_utils.rs`):**
   - Audit `safe_join` and `safe_join_new`.
   - Test against Windows-specific bypasses:
     - `\\?\C:\...` (UNC prefix stripping)
     - `file.txt::$DATA` (NTFS Alternate Data Streams)
     - `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` (DOS device names)
     - `..\\`, `../`, `..%2f`, null bytes (`%00`)
   - Test `sanitize_virtual_path`: Does it preserve isolation in nested virtual folders?
2. **Zip-Slip & Archive Decompression (`file_cmds.rs`, `backup_cmds.rs`):**
   - Audit `import_zip_file` and `import_zip_bytes`.
   - Is `zip::ZipArchive` extraction validated against Zip-Slip traversal?
   - Is there a decompression bomb guard (maximum expansion limit)?
3. **Eviction Loop Verification (`eviction.rs`):**
   - Audit the eviction loop in `evict_to_quota`.
   - Does line 153 contain an unconditional `break` that prevents the loop from evicting until `used <= low_mark`?
   - Verify sole-seed protection: can an eviction pass ever delete a blob whose only active seed is the local node?
4. **SQLite Concurrency & Multi-Network DB Isolation:**
   - Audit `switch_network_database` and WAL mode locking. Can concurrent background threads access SQLite during a database switch?

Deliver an exhaustive report with concrete exploit PoCs and production-ready code patches.
```

---

## Sub-Prompt E: Auto-Updater, Detached Execution & Supply Chain Deep Dive

```markdown
# AUDIT SUB-PROMPT E: AUTO-UPDATER, DETACHED EXECUTION & SUPPLY CHAIN (`p2pstud`)

You are a Software Supply Chain & Binary Security Auditor specializing in application auto-updaters, installer security, and OS process management.

### Scope:
- `src-tauri/src/commands/backup_cmds.rs` (updater functions)
- `src-tauri/Cargo.toml` & `Cargo.lock`
- `.github/workflows/build.yml`

### Investigation Checklist:
1. **Installer Download & Authenticity Verification (CRITICAL):**
   - Audit `download_app_update` and `install_app_update`.
   - Is the downloaded installer (`-setup.exe`, `.AppImage`, `.deb`) verified against a cryptographic signature (e.g. Minisign, GPG, Authenticode) or a hardcoded/signed SHA-256 manifest?
   - What happens if an attacker compromises GitHub release assets or performs an SSL/TLS intercept on a compromised network?
2. **Detached PowerShell Script Injection (Windows):**
   - Audit the PowerShell script constructed in `install_app_update`.
   - How are `pid`, `current_exe`, and `escaped_installer` interpolated into the script string?
   - Can single quotes (`'`), backticks, or variable substitutions (`$`) in file paths cause arbitrary command execution?
3. **Process Termination & Lock Release:**
   - Verify `Stop-Process` and `std::process::exit(0)`. Can an updater script kill unrelated processes sharing the name `p2pstud`?
4. **Linux & macOS Update Execution:**
   - Verify `chmod +x` and process spawning on Unix. Are temporary download files created in secure directories with restrictive umask?

Deliver a rigorous security analysis detailing attack vectors and a cryptographic update-signing implementation plan.
```
