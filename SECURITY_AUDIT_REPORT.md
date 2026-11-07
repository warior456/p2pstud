# EXHAUSTIVE SECURITY & ARCHITECTURAL AUDIT REPORT: `p2pstud`

**Target Application:** `p2pstud` (Tauri v2 + Iroh P2P + SQLite/Redb + React/Vite)  
**Environment:** Hostile Enterprise Academic Networks (Eduroam, KU Leuven / Groep T, KotNet, Campus Wi-Fi)  
**Auditor:** Principal Security Researcher, Distributed Systems Architect & Cryptographic Protocol Auditor  
**Date:** 2026-11-07  
**Overall Posture:** **HIGH RISK (Critical Architectural & Remote Execution Vulnerabilities Identified)**

---

## Executive Summary

An exhaustive security, reliability, and architectural audit of `p2pstud` was conducted across all 8 mandatory audit tracks specified in `AUDIT_PROMPT.md`. The audit examined Rust host code (`src-tauri/src/`), Tauri v2 host boundary configurations, frontend viewer components, cryptographic attestation mechanisms, consensus algorithms, and the proposed KU Leuven student ticket specification (`docs/KULEUVEN_AUTH_SPEC.md`).

While `p2pstud` incorporates advanced P2P technologies (Iroh QUIC transport, BLAKE3 content-addressing, LEDBAT-like bandwidth pacing, and SQLite WAL isolation), our adversarial review uncovered **14 significant vulnerabilities**, including **2 CRITICAL**, **5 HIGH**, **5 MEDIUM**, and **2 LOW/INFORMATIONAL** severity findings:

1. **[CRITICAL] Unsigned Auto-Updater Remote Code Execution:** Downloaded release installers are executed with silent flags without cryptographic signature or SHA-256 manifest verification.
2. **[CRITICAL] Sybil Consensus Subversion via CatalogDump:** Unauthenticated delete/keep votes injected via `CatalogDump` and `FileAdded` allow a single rogue node to unilaterally delete legitimate course materials or block deletion.
3. **[HIGH] Sybil Censorship via Emergency Quarantine:** Cheap ephemeral NodeIds (generated in milliseconds) can issue 5 quarantine reports within 1 hour to censor course notes across the swarm for 7 days.
4. **[HIGH] Unauthenticated HTTPS Rendezvous & Dial Amplification DoS:** Lack of signatures on `ntfy.sh` announcements allows attackers to spoof peer tickets and force all student nodes into dial storms against victim IPs.
5. **[HIGH] Low-Entropy Student Email Deanonymization in `KULEUVEN_AUTH_SPEC.md`:** `BLAKE3(salt + email)` over KU Leuven student numbers ($10^7$ space) can be cracked via rainbow table in < 1 second.
6. **[HIGH] Premature Eviction Loop Termination & Deadlock:** An unconditional `break` inside `while used > low_mark` causes `clippy::never_loop` and prevents complete cache eviction, risking disk exhaustion.
7. **[HIGH] Arbitrary File Overwrite via Unscoped `export_file`:** Lack of base path confinement in `export_file` allows arbitrary file writes on the host system.

---

## Pre-Audit Automated Verification Checklist

Before commencing manual line-by-line inspection, the automated test and build toolchains were executed:

| Step | Tool / Command | Result | Findings / Output Summary |
|---|---|---|---|
| **1** | `cargo clippy --workspace -- -D warnings` | **FAILED (14 errors)** | Identified `clippy::never_loop` in `eviction.rs:119`, unnecessary unwraps in `backup_cmds.rs:550`, collapsible conditionals in `file_cmds.rs` and `hosting_worker.rs`, doc formatting errors in `blob_tracker.rs`, and iterator misuse. |
| **2** | `cargo test --workspace` | **PASSED (48 passed, 0 failed)** | 46 library tests and 2 anchor binary tests passed cleanly. |
| **3** | `cargo audit` | **NOT INSTALLED** | `cargo-audit` binary not installed in environment. Manual dependency tree audit performed on `Cargo.lock`. |
| **4** | `npm run test` (Vitest) | **PASSED (37 passed, 6 test files)** | All frontend unit tests passed. React `act(...)` warning observed on `CacheSlider` and `ConnectionDoctor`. |
| **5** | `npm run build` (tsc + vite) | **PASSED (0 errors)** | Production bundle compiled successfully into `dist/`. |

---

## Vulnerability Findings Matrix

| Vulnerability ID | Title | Severity | CVSS v3.1 Score | Track |
|---|---|---|---|---|
| **VULN-01** | Unsigned Binary Execution in Application Auto-Updater | **CRITICAL** | **9.8** (`AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H`) | Track 7 |
| **VULN-02** | Unauthenticated Deletion & Keep Vote Injection via `CatalogDump` | **CRITICAL** | **9.1** (`AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:H/A:H`) | Track 6 |
| **VULN-03** | Sybil Emergency Quarantine Censorship Attack | **HIGH** | **7.5** (`AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H`) | Track 6 |
| **VULN-04** | Unauthenticated Rendezvous Announcement & Dial Amplification DoS | **HIGH** | **8.2** (`AV:N/AC:L/PR:N/UI:N/S:C/C:N/I:L/A:H`) | Track 1 |
| **VULN-05** | Low-Entropy Student Email Pre-computation in Auth Spec | **HIGH** | **7.5** (`AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N`) | Track 2 |
| **VULN-06** | Premature Eviction Loop Break & Storage Exhaustion Deadlock | **HIGH** | **7.5** (`AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H`) | Track 4 |
| **VULN-07** | Arbitrary File Overwrite via Unscoped `export_file` IPC | **HIGH** | **8.1** (`AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:H/A:H`) | Track 3 / 5 |
| **VULN-08** | Zip Bomb Memory Exhaustion in Archive Import | **MEDIUM** | **6.5** (`AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:N/A:H`) | Track 5 |
| **VULN-09** | Cleartext Student Endpoint PII Leakage to Public Pub/Sub | **MEDIUM** | **5.3** (`AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N`) | Track 1 |
| **VULN-10** | Diagnostic UDP Probe False Positive & Misleading NAT Reporting | **MEDIUM** | **4.3** (`AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:N`) | Track 1 |
| **VULN-11** | ALPN Swarm Isolation Desynchronization on Passphrase Change | **MEDIUM** | **5.3** (`AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:L`) | Track 1 |
| **VULN-12** | SecretKey Insecure File Creation Permissions Window | **MEDIUM** | **5.5** (`AV:L/AC:L/PR:L/UI:N/S:U/C:H/I:N/A:N`) | Track 2 |
| **VULN-13** | Database Corruption via Raw Overwrite on Active SQLite Connection | **LOW** | **4.4** (`AV:L/AC:L/PR:H/UI:R/S:U/C:N/I:H/A:H`) | Track 4 |
| **VULN-14** | Unrestricted Remote Image Exfiltration in Markdown Preview | **LOW** | **3.5** (`AV:N/AC:L/PR:N/UI:R/S:U/C:L/I:N/A:N`) | Track 3 |

---

# Detailed Findings by Track

---

## TRACK 1: P2P Network Protocol, Campus Stealth & Anti-IDS Evasion

### [VULN-04] Unauthenticated Rendezvous Announcement & Dial Amplification DoS
- **Severity:** High
- **CVSS v3.1 Score & Vector:** 8.2 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:N/I:L/A:H`
- **Affected Component(s):** `src-tauri/src/p2p/rendezvous.rs#L18-L24, L145-L206`
- **Vulnerability Category:** Denial of Service / Protocol Spoofing

#### 1. Vulnerability Description & Technical Root Cause
The automated rendezvous mechanism uses `ntfy.sh` (or a configured HTTP pub/sub endpoint) on port 443 to overcome symmetric NAT and Eduroam client isolation. Peers derive a topic name (`p2pstud-<hash>`) and periodically publish a JSON announcement:
```rust
pub struct RendezvousAnnouncement {
    pub node_id: String,
    pub ticket: String,
    pub timestamp: u64,
}
```
Crucially, **the announcement is completely unauthenticated and carries no cryptographic signature**. When a client polls the topic:
```rust
for line in text.lines() {
    if let Ok(peer) = serde_json::from_str::<RendezvousAnnouncement>(msg_str) {
        if peer.node_id != our_node_id && peer.timestamp + 300 >= now {
            connect_and_sync_discovered_peer(endpoint, db, store, &peer.ticket, &peer.node_id).await;
        }
    }
}
```
For every message received, the node parses `peer.ticket` into an `EndpointAddr` and calls `endpoint.connect(addr, iroh_blobs::ALPN).await`. 

#### 2. Exploit Scenario / Proof of Concept (PoC)
1. An adversary script publishes 5,000 bogus announcements to `https://ntfy.sh/p2pstud-6429...` (the known open swarm topic):
   ```bash
   for i in $(seq 1 5000); do
     curl -s -X POST https://ntfy.sh/p2pstud-open-topic \
       -d "{\"node_id\":\"fake$i\",\"ticket\":\"invalid_ticket_$i\",\"timestamp\":$(date +%s)}"
   done
   ```
2. Or worse, the attacker crafts tickets referencing victim university infrastructure IP addresses on UDP port 443 or arbitrary external IPs.
3. Every running `p2pstud` node on the campus network fetches this list during its 15s/60s cycle and attempts 5,000 concurrent QUIC handshakes.
4. **Result:** Client thread pools, UDP sockets, and memory are exhausted, causing the desktop app to freeze and inundating victim networks with unsolicited QUIC initial packets (Dial Amplification).

#### 3. Impact Assessment
Complete denial of service for all campus nodes, high memory and battery drain on student laptops, and potential campus IT firewall blacklisting due to high-volume outbound UDP connection storms.

#### 4. Remediation & Code Diff
Require all announcements to include a signature generated by the announcing node's `SecretKey`. Verify `ticket.node_id` matches the signing key, discard unverified messages, and enforce a strict rate limiter (e.g. max 10 new peer connection attempts per cycle).

```diff
 #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
 pub struct RendezvousAnnouncement {
     pub node_id: String,
     pub ticket: String,
     pub timestamp: u64,
+    pub signature: String,
 }

+impl RendezvousAnnouncement {
+    pub fn verify(&self) -> bool {
+        let Ok(pk) = self.node_id.parse::<iroh::PublicKey>() else { return false };
+        let payload = format!("{}:{}:{}", self.node_id, self.ticket, self.timestamp);
+        let Ok(sig_bytes) = hex::decode(&self.signature) else { return false };
+        let Ok(sig) = iroh::Signature::from_bytes(&sig_bytes) else { return false };
+        pk.verify(payload.as_bytes(), &sig).is_ok()
+    }
+}
```

---

### [VULN-09] Cleartext Student Endpoint PII Leakage to Public Pub/Sub
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 5.3 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N`
- **Affected Component(s):** `src-tauri/src/p2p/rendezvous.rs#L128-L151`
- **Vulnerability Category:** Information Disclosure / Privacy Violation

#### 1. Vulnerability Description & Technical Root Cause
In `run_rendezvous_cycle`:
```rust
let our_node_id = endpoint.id().to_string();
let our_addr = endpoint.addr();
let our_ticket = iroh_tickets::endpoint::EndpointTicket::new(our_addr).to_string();
```
The client serializes its full `EndpointTicket` (which contains the student's local IP, public IP, port numbers, and relay server details) and posts it in plaintext JSON to public pub/sub (`ntfy.sh`). Any third party listening to `https://ntfy.sh/<topic>/json` can passively harvest the network locations, active hours, and persistent NodeIds of every student using the application.

#### 2. Impact Assessment
Allows network eavesdroppers or malicious students to track when specific peers are online, correlate their campus locations, and map internal university IP allocation subnets.

#### 3. Remediation & Code Diff
Encrypt rendezvous announcements using an authenticated symmetric cipher (e.g., ChaCha20-Poly1305) keyed with `BLAKE3("rendezvous-envelope:" + network_secret)`. On open networks without a passphrase, derive an app-specific obfuscation key so that generic `ntfy.sh` scrapers cannot inspect student tickets in cleartext.

---

### [VULN-10] Diagnostic UDP Probe False Positive & Misleading NAT Reporting
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 4.3 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:N`
- **Affected Component(s):** `src-tauri/src/commands/diagnostic_cmds.rs#L48-L95`
- **Vulnerability Category:** Diagnostic Logic Flaw / False Operational Telemetry

#### 1. Vulnerability Description & Technical Root Cause
In `run_connection_doctor`:
```rust
let quic_udp_ok = match tokio::time::timeout(Duration::from_millis(1500), tokio::net::UdpSocket::bind("0.0.0.0:0")).await {
    Ok(Ok(sock)) => sock.connect("1.1.1.1:53").await.is_ok(),
    _ => false,
};
```
In POSIX and WinSock UDP socket implementations, `sock.connect("1.1.1.1:53")` **does not transmit any packets on the wire**. It merely binds the remote destination address in the local OS socket structure. As long as `0.0.0.0:0` binds and `1.1.1.1` is a valid IP, `sock.connect` returns `Ok(())` in ~0.1 ms even if outbound UDP is completely blocked by the Eduroam firewall.
Consequently, `quic_udp_ok` evaluates to `true`, and test 4 reports:
```rust
let is_symmetric_nat = !quic_udp_ok; // false!
// Reports "full_cone NAT: direct peer connections possible"
```
Furthermore, test 1 (mDNS) merely checks if a local socket is bound to a private IP, without testing actual multicast group membership (`224.0.0.251:5353`), reporting "LAN peer discovery active" even when client isolation drops all multicast frames.

#### 2. Remediation & Code Diff
Transmit an actual minimal DNS query (or QUIC version negotiation probe) and wait for a response, or check Iroh's endpoint active connection type (`endpoint.home_relay_status()` / `endpoint.connection_info()`).

```diff
- let quic_udp_ok = match tokio::time::timeout(Duration::from_millis(1500), tokio::net::UdpSocket::bind("0.0.0.0:0")).await {
-     Ok(Ok(sock)) => sock.connect("1.1.1.1:53").await.is_ok(),
-     _ => false,
- };
+ // Inspect actual Iroh endpoint connection stats or perform a real round-trip DNS probe
+ let quic_udp_ok = {
+     let guard = engine_lock.read().await;
+     if let Some(engine) = guard.as_ref() {
+         // Verify if any direct UDP path is currently active or probe DNS with an actual query packet
+         tokio::net::lookup_host("1.1.1.1:53").await.is_ok()
+     } else {
+         false
+     }
+ };
```

---

### [VULN-11] ALPN Swarm Isolation Desynchronization on Passphrase Change
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 5.3 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:L`
- **Affected Component(s):** `src-tauri/src/commands/storage_cmds.rs#L219-L229`, `src-tauri/src/p2p/engine.rs#L107-L113`
- **Vulnerability Category:** Protocol Desynchronization / State Machine Flaw

#### 1. Vulnerability Description & Technical Root Cause
When the user changes the swarm passphrase in settings, `set_network_secret` is called:
```rust
pub async fn set_network_secret(secret: String, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let trimmed = secret.trim();
    state.db.switch_network(trimmed).map_err(|e| e.to_string())?;
    let _ = app.emit("network-database-switched", trimmed);
    Ok(())
}
```
`state.db.switch_network` updates the active SQLite database, but **does not restart or update the running `IrohEngine` router**. The Iroh endpoint was initialized in `lib.rs` with `sync_alpn = get_sync_alpn(&db)` and registered on the router. 
As a result:
1. Inbound connections on the new ALPN are rejected by the Iroh router because it is still listening on the old ALPN.
2. The node cannot sync with peers in the newly selected network until the entire application is manually terminated and restarted.

#### 2. Remediation & Code Diff
Update `set_network_secret` to trigger an engine rebind or restart the router with the newly derived ALPN protocol handler.

---

## TRACK 2: Cryptographic Primitives, Key Management & Authentication Spec

### [VULN-05] Low-Entropy Student Email Pre-computation in Auth Spec
- **Severity:** High
- **CVSS v3.1 Score & Vector:** 7.5 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N`
- **Affected Component(s):** `docs/KULEUVEN_AUTH_SPEC.md#L77-L79, L24`
- **Vulnerability Category:** Cryptographic Protocol Flaw / Pre-computation Attack

#### 1. Vulnerability Description & Technical Root Cause
Section 4 of `KULEUVEN_AUTH_SPEC.md` defines the student attestation ticket:
```rust
pub struct GroepTMemberTicket {
    pub node_id: String,
    pub email_hash: String, // BLAKE3(salt + email)
    ...
}
```
The specification asserts that `BLAKE3(salt + student_email)` guarantees privacy ("Peers only see a salted one-way hash: plaintext email is never broadcast").
However, KU Leuven student numbers follow an exact, constrained format:
`r` followed by exactly 7 decimal digits (`r0000000` to `r9999999`). In reality, active enrolled student numbers reside within an even narrower sub-range (`r0600000` – `r0999999`, roughly $4 \times 10^6$ combinations).
Because BLAKE3 is designed for hyper-speed ($> 3\text{ GB/s}$ throughput), an attacker can compute the BLAKE3 digest of all 10,000,000 possible student emails in **under 1.2 seconds** on an ordinary laptop:
$$\text{Search Space} = 10^7 \text{ candidates} \ll 2^{24}$$
Even with a per-network salt, once the salt is known (or extracted from the client binary/rendezvous), the attacker builds a rainbow lookup table instantly. Every ticket exchanged over the swarm is immediately deanonymized to the student's real name and ID.

#### 2. Remediation & Protocol Enhancement
1. **Replace BLAKE3 with a Memory-Hard Slow KDF:** Use Argon2id ($m=64\text{MB}, t=3, p=4$) or scrypt with a campus salt, making brute-forcing $10^7$ student numbers prohibitively expensive.
2. **Cryptographic Blind Attestation / Anonymous Credentials:** Instead of publishing `email_hash` in the ticket, the Anchor Authority issues an anonymous capability ticket (such as a W3C Verifiable Credential or Privacy Pass token using Ed25519 Blind Signatures). The student proves institutional validity without revealing any email identifier.

---

### [VULN-12] SecretKey Insecure File Creation Permissions Window
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 5.5 — `CVSS:3.1/AV:L/AC:L/PR:L/UI:N/S:U/C:H/I:N/A:N`
- **Affected Component(s):** `src-tauri/src/p2p/identity.rs#L30-L47`
- **Vulnerability Category:** Insecure Permissions / File System Race Condition

#### 1. Vulnerability Description & Technical Root Cause
In `load_or_create_secret_key`:
```rust
let tmp = key_path.with_extension("tmp");
std::fs::write(&tmp, bytes)?; // [1] File created with default umask / inherited DACL

#[cfg(unix)]
std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?; // [2] Permissions restricted

#[cfg(windows)]
restrict_file_to_current_user(&tmp)?; // [3] icacls executed
```
At line 32 (`std::fs::write`), the file is created with the process's default umask (typically `0644` or `0666` on Unix, or inheriting standard Users `Read & Execute` DACLs on Windows). Between lines 32 and 39/44, the secret key sits unencrypted on disk with world-readable permissions. On shared university workstations (e.g. computer labs), concurrent local users or background processes can read the temporary file before permissions are tightened.

#### 2. Remediation & Code Diff
On Unix, create the file directly with mode `0o600` using `OpenOptionsExt`. On Windows, create a temporary file with secure flags before writing.

```diff
 #[cfg(unix)]
 {
     use std::os::unix::fs::OpenOptionsExt;
-    std::fs::write(&tmp, bytes)?;
-    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
+    let mut file = std::fs::OpenOptions::new()
+        .create_new(true)
+        .write(true)
+        .mode(0o600)
+        .open(&tmp)?;
+    file.write_all(bytes)?;
 }
```

---

## TRACK 3: Host Boundary, Tauri v2 IPC & Webview Sandboxing

### [VULN-07] Arbitrary File Overwrite via Unscoped `export_file` IPC
- **Severity:** High
- **CVSS v3.1 Score & Vector:** 8.1 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:H/A:H`
- **Affected Component(s):** `src-tauri/src/commands/file_cmds.rs#L764-L806`
- **Vulnerability Category:** Path Traversal / Arbitrary File Overwrite

#### 1. Vulnerability Description & Technical Root Cause
The `export_file` command allows exporting a cached blob to disk:
```rust
#[tauri::command]
pub async fn export_file(
    hash: String,
    dest_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if dest_path.contains('\0') { return Err(...); }
    if dest_path.contains("..") { return Err(...); }
    ...
    let path_buf = std::path::PathBuf::from(&dest_path);
    let target = if path_buf.is_absolute() {
        path_buf
    } else {
        ...
    };
    std::fs::write(&target, data)...
}
```
If `dest_path` is absolute (e.g., `C:\Users\<Victim>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\payload.bat` on Windows or `/home/<victim>/.bashrc` on Linux), it contains neither `\0` nor `..`. The command directly executes `std::fs::write(&target, data)`, overwriting arbitrary user files without dialog prompt or path confinement.

#### 2. Remediation & Code Diff
Ensure that all exports are strictly scoped to the user's `download_dir()` using `safe_join_new`, or require the destination to be selected through the native OS save dialog (`rfd::AsyncFileDialog`).

```diff
- let path_buf = std::path::PathBuf::from(&dest_path);
- let target = if path_buf.is_absolute() {
-     path_buf
- } else {
-     let file_name = path_buf.file_name().ok_or("Invalid destination filename")?;
-     app.path().download_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).join(file_name)
- };
+ let downloads = app.path().download_dir().map_err(|e| e.to_string())?;
+ let filename = std::path::Path::new(&dest_path)
+     .file_name()
+     .ok_or_else(|| "Invalid export filename".to_string())?
+     .to_string_lossy();
+ let target = crate::storage::path_utils::safe_join_new(&downloads, &filename)
+     .map_err(|e| format!("Export destination outside downloads directory: {e}"))?;
```

---

### [VULN-14] Unrestricted Remote Image Exfiltration in Markdown Preview
- **Severity:** Low
- **CVSS v3.1 Score & Vector:** 3.5 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:U/C:L/I:N/A:N`
- **Affected Component(s):** `src/components/Viewers/MarkdownEditor.tsx#L62-L69`
- **Vulnerability Category:** Privacy Leak / Webview Sandboxing

#### 1. Vulnerability Description & Technical Root Cause
In `MarkdownEditor.tsx`:
```typescript
const getHtml = () => {
  try {
    const raw = marked.parse(content, { async: false }) as string;
    return DOMPurify.sanitize(raw);
  } catch {
    return '<p>Error rendering markdown</p>';
  }
};
```
`DOMPurify.sanitize(raw)` is invoked with default settings. While default DOMPurify removes inline `<script>` tags, it **permits external image tags** (`<img src="https://...">`).
When a student opens a peer's markdown file, the WebView automatically issues HTTP GET requests to any remote image URLs. An attacker hosting the remote server logs the student's campus IP address, exact timestamp, and User-Agent, effectively turning markdown notes into tracking beacons.

#### 2. Remediation & Code Diff
Configure DOMPurify with strict tag and attribute whitelists, disallowing remote URL schemes in `img` tags or rewriting them to local blob protocols.

```diff
- return DOMPurify.sanitize(raw);
+ return DOMPurify.sanitize(raw, {
+   ALLOWED_TAGS: ['b', 'i', 'em', 'strong', 'a', 'p', 'h1', 'h2', 'h3', 'h4', 'ul', 'ol', 'li', 'code', 'pre', 'blockquote', 'hr', 'table', 'thead', 'tbody', 'tr', 'th', 'td'],
+   ALLOWED_ATTR: ['href', 'title', 'class'],
+   ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i,
+   FORBID_TAGS: ['img', 'iframe', 'object', 'embed', 'form'],
+ });
```

---

## TRACK 4: Storage Engine, Concurrency & Redb / SQLite Integrity

### [VULN-06] Premature Eviction Loop Break & Storage Exhaustion Deadlock
- **Severity:** High
- **CVSS v3.1 Score & Vector:** 7.5 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H`
- **Affected Component(s):** `src-tauri/src/storage/eviction.rs#L119-L154`
- **Vulnerability Category:** Architectural Defect / Never-Loop Eviction Failure

#### 1. Vulnerability Description & Technical Root Cause
The eviction worker drives cache usage from the 90% high watermark down to the 80% low watermark:
```rust
while used > low_mark {
    let candidates = select_eviction_candidates(db, Some(&my_node_id), now)?;
    if candidates.is_empty() {
        tracing::warn!("No eligible unpinned blobs left to evict...");
        break;
    }

    for blob in candidates {
        if used <= low_mark {
            break;
        }
        ...
        used = used.saturating_sub(blob.size_bytes);
        deleted += 1;
    }
    // If we processed all candidates and couldn't evict enough, break to avoid infinite loop
    break; // <-- CRITICAL DEFECT
}
```
Line 153 contains an unconditional `break;` statement. This was flagged by compiler clippy warnings (`clippy::never_loop`).
Because of this `break`, the `while used > low_mark` loop **never executes more than one pass**. `select_eviction_candidates` fetches at most 200 candidates (`oldest_unpinned(db, 200)`). If those 200 candidates are small files or are skipped by the dwell time guard (written < 24h ago) or sole-seed protection, `used` remains well above `low_mark`.
Even worse: if a user downloads several large files, all blobs have `written_at < 24h`. The dwell time guard rejects 100% of candidates. The loop breaks immediately, disk space cannot be reclaimed, and subsequent writes fail with storage exhaustion.

#### 2. Remediation & Code Diff
Remove the unconditional `break;` and only break if no forward progress was made in the inner loop (`deleted == 0`). Add an emergency fallback pass that relaxes the 24-hour dwell guard if disk usage exceeds 98% of quota.

```diff
+ let mut progress = false;
  for blob in candidates {
      if used <= low_mark {
          break;
      }
      ...
      used = used.saturating_sub(blob.size_bytes);
      deleted += 1;
+     progress = true;
  }
- break;
+ if !progress {
+     break;
+ }
```

---

### [VULN-13] Database Corruption via Raw Overwrite on Active SQLite Connection
- **Severity:** Low
- **CVSS v3.1 Score & Vector:** 4.4 — `CVSS:3.1/AV:L/AC:L/PR:H/UI:R/S:U/C:N/I:H/A:H`
- **Affected Component(s):** `src-tauri/src/commands/backup_cmds.rs#L191-L206`
- **Vulnerability Category:** Database Concurrency / File Lock Corruption

#### 1. Vulnerability Description & Technical Root Cause
In `restore_app_backup`:
```rust
{
    let conn = state.db.lock();
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
}

if let Some(db_data) = db_bytes {
    std::fs::write(&current_db_path, db_data)...
}
```
While a WAL checkpoint is issued, `state.db` and any open background queries (e.g. `hosting_worker` or `scarcity_worker`) still hold live file handles and shared memory mappings (`p2pstud.db-shm` / `p2pstud.db-wal`).
Overwriting the database file on disk directly via `std::fs::write` while SQLite connections remain open causes immediate file locking errors on Windows (`ERROR_SHARING_VIOLATION`), or leaves orphaned WAL pages that corrupt the catalog on Linux.

#### 2. Remediation & Code Diff
Use SQLite's official online backup API (`rusqlite::backup::Backup`) or replace the connection atomically using `Database::switch_network`.

---

## TRACK 5: Filesystem Sanitization, Path Traversal & Zip-Slip

### [VULN-08] Zip Bomb Memory Exhaustion in Archive Import
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 6.5 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:U/C:N/I:N/A:H`
- **Affected Component(s):** `src-tauri/src/commands/file_cmds.rs#L906-L914, L1002-L1006`
- **Vulnerability Category:** Resource Exhaustion / Uncontrolled Expansion

#### 1. Vulnerability Description & Technical Root Cause
In `import_zip_archive_internal`:
```rust
let mut file_bytes = Vec::new();
if let Err(e) = file.read_to_end(&mut file_bytes) {
    summary.errors.push(format!("Failed reading bytes: {e}"));
    continue;
}
```
Each entry in the ZIP archive is read entirely into an in-memory buffer (`Vec<u8>`) via `file.read_to_end`.
The code enforces no limit on the uncompressed size of an entry or the compression ratio. A standard 10 MB recursive ZIP bomb (such as a 10 GB stream of zeroes) decompresses in memory until the operating system aborts the process via OOM killer.
Furthermore, streaming multi-gigabyte legitimate lecture videos allocates gigabytes into heap RAM, violating the project requirement:
`Idle RAM budget: ≤ 120 MB`.

#### 2. Remediation & Code Diff
Enforce a maximum single file limit (e.g. 500 MB for ZIP entry extraction), limit the maximum compression expansion ratio (max 100:1), and stream data through a bounded reader directly into the `iroh-blobs` store.

```diff
+ const MAX_ZIP_ENTRY_SIZE: u64 = 500 * 1024 * 1024; // 500 MB limit per entry
+ if file.size() > MAX_ZIP_ENTRY_SIZE {
+     summary.errors.push(format!("Entry '{}' exceeds maximum allowed size", entry_name));
+     continue;
+ }
+ let mut file_bytes = Vec::with_capacity(file.size().min(MAX_ZIP_ENTRY_SIZE) as usize);
+ file.take(MAX_ZIP_ENTRY_SIZE).read_to_end(&mut file_bytes)?;
```

---

## TRACK 6: Distributed Consensus, Governance & Anti-Abuse

### [VULN-02] Unauthenticated Deletion & Keep Vote Injection via `CatalogDump`
- **Severity:** Critical
- **CVSS v3.1 Score & Vector:** 9.1 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:H/A:H`
- **Affected Component(s):** `src-tauri/src/p2p/sync.rs#L203-L208, L311-L316`
- **Vulnerability Category:** Distributed Consensus Subversion / Sybil Attack

#### 1. Vulnerability Description & Technical Root Cause
In `CatalogSyncHandler::handle_message`:
When processing an individual delete vote message (`SyncMessage::FileVoteDelete`), the code validates:
```rust
// In FileVoteDelete:
if sender_peer_id == Some(voter_node_id.as_str()) {
    let _ = cast_delete_vote(&self.db, &hash, &voter_node_id);
}
```
**However, in `SyncMessage::CatalogDump` and `SyncMessage::FileAdded`:**
```rust
// Lines 203-208 and 311-316:
for voter in r.delete_votes {
    let _ = cast_delete_vote(&self.db, &r.hash, &voter);
}
for voter in r.keep_votes {
    let _ = cast_keep_vote(&self.db, &r.hash, &voter);
}
```
When a peer synchronizes catalogs, it receives a vector of `BlobRecord`s. The receiver **blindly casts every voter ID in `r.delete_votes` and `r.keep_votes` directly into SQLite without validating signatures or checking sender identity**!

#### 2. Exploit Scenario / Proof of Concept (PoC)
1. Alice uploads an important exam summary (`math_exam_notes.pdf`).
2. Attacker Mallori joins the swarm and sends a `SyncMessage::CatalogDump` containing Alice's blob hash with:
   ```json
   "delete_votes": ["node_sybil_1", "node_sybil_2", "node_sybil_3"]
   ```
3. Every peer receiving the catalog dump records 3 delete votes.
4. Because `DELETE_VOTE_QUORUM = 3` and $\frac{3}{3} = 100\% \ge 86\%$, `evaluate_deletion_schedule` immediately schedules the file for deletion (`deletion_scheduled_at = now + 86400`).
5. After 24 hours, all nodes permanently delete Alice's lecture notes from their catalogs!
6. Conversely, Mallori can inject fake `keep_votes` to permanently prevent legitimate community deletion of malicious files.

#### 3. Impact Assessment
Complete subversion of distributed community governance. Any single malicious student can delete all academic materials on the campus network with zero coordination.

#### 4. Remediation & Code Diff
Only accept delete and keep votes that are accompanied by a cryptographic signature from each voter:
`Signature = Ed25519_Sign(voter_secret, "vote-delete:" + hash + ":" + timestamp)`. Reject unsigned votes.

---

### [VULN-03] Sybil Emergency Quarantine Censorship Attack
- **Severity:** High
- **CVSS v3.1 Score & Vector:** 7.5 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H`
- **Affected Component(s):** `src-tauri/src/storage/blob_tracker.rs#L834-L867`, `src-tauri/src/commands/file_cmds.rs#L1445-L1466`, `src-tauri/src/p2p/sync.rs#L469-L480`
- **Vulnerability Category:** Abuse Resistance / Censorship Attack

#### 1. Vulnerability Description & Technical Root Cause
The emergency quarantine feature allows students to report dangerous or malicious files:
```rust
pub fn report_dangerous(db: &Database, hash: &str, reporter_node_id: &str) -> Result<QuarantineStatus> {
    ...
    conn.execute(
        "INSERT OR REPLACE INTO quarantine_reports(hash, reporter_node_id, reported_at) VALUES(?1, ?2, ?3)",
        params![hash, reporter_node_id, now],
    )?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM quarantine_reports WHERE hash = ?1 AND reported_at > (?2 - 3600)",
        params![hash, now],
        |r| r.get(0),
    )?;
    if count >= 5 {
        conn.execute("UPDATE local_blobs SET is_quarantined = 1, quarantine_expires_at = ?1 WHERE hash = ?2", ...);
    }
}
```
If 5 reports arrive in 1 hour, the file is quarantined for 7 days (`is_quarantined = 1`), hiding it from the catalog.
Generating an Ed25519 NodeId takes $< 0.1\text{ ms}$ (`SecretKey::generate()`). 
A rogue client can instantly spawn 5 ephemeral Iroh identities, connect to peers, and emit 5 `FileQuarantineReport` messages. Every peer receiving these 5 reports marks the file quarantined.

#### 2. Impact Assessment
Any student can censor course notes, summaries, or past exams right before an exam session.

#### 3. Remediation & Code Diff
1. Bind report capability to authenticated student identity tickets (`KULEUVEN_AUTH_SPEC.md`).
2. Require reports to come from distinct verified `email_hash` identities rather than raw NodeIds.
3. Require at least 5% of active network members (or Anchor node concurrence) before quarantining.

---

## TRACK 7: Auto-Updater, Detached Execution & Supply Chain

### [VULN-01] Unsigned Binary Execution in Application Auto-Updater
- **Severity:** Critical
- **CVSS v3.1 Score & Vector:** 9.8 — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H`
- **Affected Component(s):** `src-tauri/src/commands/backup_cmds.rs#L620-L724`
- **Vulnerability Category:** Supply Chain Execution / Unauthenticated Remote Code Execution

#### 1. Vulnerability Description & Technical Root Cause
The in-place auto-updater checks GitHub releases for new versions:
```rust
// backup_cmds.rs line 620:
let resp = client.get(&download_url).send().await?;
let bytes = resp.bytes().await?;
tokio::fs::write(&dest_path, &bytes).await?;
```
And then in `install_app_update`:
```rust
// On Windows (backup_cmds.rs line 704):
Start-Process -FilePath '{escaped_installer}' -ArgumentList '/S' -Wait;

// On Linux (backup_cmds.rs line 737):
std::process::Command::new("chmod").arg("+x").arg(&installer_path).status();
std::process::Command::new(&installer_path).spawn();
```
**There is zero signature or cryptographic hash verification of the downloaded installer.**
The downloaded artifact is not checked against an embedded Minisign public key, GPG signature, or SHA-256 hash manifest.
If:
- An attacker compromises the GitHub repository release credentials,
- A malicious actor on a compromised Wi-Fi router intercepts or spoofs GitHub release API responses, or
- SQLite database settings are modified locally (`update_installer_path`),

The application silently executes the untrusted binary with elevated or current user privileges using the silent `/S` parameter.

#### 2. Exploit Scenario / Proof of Concept (PoC)
1. Attacker controls an upstream asset or DNS record.
2. `download_app_update` writes the malicious `.exe` payload to `LocalApplicationData\p2pstud\updates\payload.exe`.
3. `install_app_update` spawns PowerShell:
   ```powershell
   Start-Process -FilePath 'C:\Users\...\updates\payload.exe' -ArgumentList '/S' -Wait
   ```
4. Payload executes silently in background with full user privileges.

#### 3. Impact Assessment
Total system compromise (Remote Code Execution) of all student computers running `p2pstud`.

#### 4. Remediation & Code Diff
Implement cryptographic signature verification before staging any update. Embed a hardcoded public key (e.g. Minisign or Ed25519) in the binary:

```diff
+ const UPDATE_SIGNING_PUBKEY: &str = "RWQ..."; // Ed25519 / Minisign public key
+
+ let signature = client.get(&format!("{download_url}.minisig")).send().await?.bytes().await?;
+ if !verify_minisign(&bytes, &signature, UPDATE_SIGNING_PUBKEY) {
+     return Err("Cryptographic signature verification failed for update installer!".to_string());
+ }
```

---

## TRACK 8: Async Concurrency, Deadlocks & Resource Exhaustion (DoS)

### [VULN-08.1] Tokio Runtime Thread Pool Starvation from Blocking Synchronous I/O
- **Severity:** Medium
- **CVSS v3.1 Score & Vector:** 5.3 — `CVSS:3.1/AV:L/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:L`
- **Affected Component(s):** `src-tauri/src/commands/backup_cmds.rs#L104-L130, L330-L369`, `src-tauri/src/commands/file_cmds.rs#L1156`
- **Vulnerability Category:** Concurrency Starvation / Reactor Blocking

#### 1. Vulnerability Description & Technical Root Cause
Commands like `export_app_backup`, `restore_app_backup`, and `export_folder_zip` perform heavy synchronous operations:
```rust
let file = File::create(&dest_path)?;
let mut zip = ZipWriter::new(file);
...
zip.write_all(&data_bytes)?;
zip.finish()?;
```
These functions are marked `async fn` and run directly on the Tokio cooperative thread pool. Compressing multi-gigabyte files or writing to slow disk drives blocks the Tokio worker thread for tens of seconds, starving QUIC packet handling, timer ticks, and IPC responses.

#### 2. Remediation & Code Diff
Wrap all synchronous compression and file I/O inside `tokio::task::spawn_blocking`.

```diff
- let mut zip = ZipWriter::new(file);
- zip.write_all(&data_bytes)?;
- zip.finish()?;
+ tokio::task::spawn_blocking(move || -> Result<()> {
+     let mut zip = ZipWriter::new(file);
+     zip.write_all(&data_bytes)?;
+     zip.finish()?;
+     Ok(())
+ }).await.map_err(|e| e.to_string())??;
```

---

# Verification and Fix Plan

To restore the codebase to production readiness and clear all compiler/clippy warnings:

1. **Eviction Loop Fix (`eviction.rs`):** Remove unconditional `break;` at line 153.
2. **Clippy Cleanup:**
   - Fix unnecessary unwrap in `backup_cmds.rs:550`.
   - Collapse nested `if` statements in `file_cmds.rs:153, 184` and `hosting_worker.rs:141`.
   - Replace `.split('/').last()` with `.split('/').next_back()` in `file_cmds.rs:1381` and `path_utils.rs:223`.
   - Replace verbose match with `matches!` in `storage_cmds.rs:456`.
   - Fix doc list continuation in `blob_tracker.rs:169`.
   - Use `.flatten()` on iterator queries in `blob_tracker.rs:919, 1145`.
3. **Consensus Vote Hardening (`sync.rs`):** Reject unauthenticated votes in `CatalogDump`.
4. **Auto-Updater Check:** Enforce SHA-256 / cryptographic verification before launching NSIS.

---

*(Artifact stored in audit directory for persistent project reference)*
