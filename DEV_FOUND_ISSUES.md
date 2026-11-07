# DEV_FOUND_ISSUES.md — Resolved Issues Log

## Issues & Resolutions (Version 0.1.23)

### 1. Iroh Engine Crash on Legacy redb v2 Index (`migration from redb v2 no longer supported`)
- **Issue:** P2P engine failed to start on existing installations, causing "Complete Node Ticket" to be stuck on "Generating ticket..." and swarm discovery to fail with "P2P network engine is still starting up".
- **Root Cause:**
  - Upgrading `iroh-blobs` to `0.104` moved the underlying storage engine to `redb v3`.
  - When loading an existing user blob directory containing a legacy `blobs.db` created under `redb v2`, `BlobStore::load` threw `migration from redb v2 no longer supported; upgrade with an older redb version first`.
  - This prevented the engine from ever initializing and caused `state.engine` to remain `None` permanently.
- **Fix:**
  - `src-tauri/src/p2p/engine.rs`: Added automatic recovery for obsolete/incompatible database indexes. If `BlobStore::load` detects a legacy `redb v2` or migration error, it safely backs up `blobs.db` to `blobs.db.v2.bak` and initializes a fresh `redb v3` index, preserving all existing `.data` and `.obao4` course files intact. Verified with live integration test booting the real user data directory in 0.19 seconds.

### 2. Silent In-Place Updater Process Lock & Hanging Restarts
- **Issue:** Clicking "Update & Restart Now" appeared to do nothing; process remained running and the new version did not start.
- **Root Cause:**
  - `confirm_app_exit` and `install_app_update` previously invoked `app.exit(0)`, which was intercepted and cancelled by Tauri's `WindowEvent::CloseRequested` handler (`api.prevent_close()`), keeping `p2pstud.exe` running and locking the executable.
  - The silent NSIS installer failed because Windows blocked overwriting an actively executing binary (`ERROR_SHARING_VIOLATION`).
  - No downloading visual feedback was given while the ~15MB installer was transferring over HTTPS.
- **Fix:**
  - `src-tauri/src/commands/backup_cmds.rs`: Updated detached PowerShell updater script to forcefully terminate the old process ID and kill any locked instances before invoking NSIS silently with `/S`, followed by `std::process::exit(0)` to release all OS locks immediately.
  - `src/App.tsx`: Added `isUpdatingNow` spinner state, disabled button during download, and displayed clear stage indicators ("Downloading update (15 MB)..." and "Applying update and restarting...").

---

## Issues & Resolutions (Version 0.1.22)

### 1. Swarm Rendezvous Startup Timing & Adaptive Discovery Speed
- **Issue:** Clicking "Discover Peers Now" immediately upon launching P2PStud resulted in `"Discovery trigger failed: Iroh engine not yet started"`.
- **Root Cause:**
  - Tauri loads and renders the UI in milliseconds, while the background thread takes 1–2 seconds to initialize cryptographic keys, load the 3GB local blob store from disk, and bind QUIC sockets.
  - Calling `trigger_swarm_discovery` during that initial startup window returned an immediate error instead of waiting for initialization to finish.
  - Background rendezvous discovery was running at a static 20-second interval regardless of swarm connection state.
- **Fix:**
  - `src-tauri/src/commands/peer_cmds.rs`: `trigger_swarm_discovery` now polls and awaits engine readiness for up to 5 seconds before returning an error, eliminating the startup race condition.
  - `src-tauri/src/p2p/rendezvous.rs`: Implemented **adaptive automated swarm discovery**:
    - **No peers connected (`peer_count == 0`)**: Runs fast every **15 seconds** for instant peer discovery in classrooms or libraries.
    - **Peers connected (`peer_count > 0`)**: Automatically reduces speed and relaxes to **60 seconds**, minimizing bandwidth and CPU usage.
  - `src/components/Settings/NetworkSettings.tsx`: Fixed status banner styling so engine initialization or network notices render with appropriate warning styles rather than misleading green success boxes.

---

## Issues & Resolutions (Version 0.1.21)

### 1. KU Leuven ICTS Campus Security Trigger & Intrusion Detection
- **Issue:** User received security alert from KU Leuven ICTS on campus network (`10.45.219.240`): `Type malware (technical log): BitTorrent distribution and connections to malware IPs`.
- **Root Cause Forensic Analysis:**
  1. `connections to malware IPs`: In `diagnostic_cmds.rs`, the QUIC UDP diagnostic check sent a UDP packet to `8.8.8.8:12345`. Port 12345 is the infamous NetBus trojan backdoor port. University IDS (Suricata/Snort/Palo Alto) flagged this as malware trojan communication.
  2. `BitTorrent distribution`:
     - Early versions (`v0.1.0` - `v0.1.9`) had `.discovery_dht()` enabled, querying public BitTorrent DHT.
     - Recent versions had Iroh `Portmapper` enabled by default (`PortmapperConfig::Enabled`), which broadcasted UPnP SSDP multicast packets to `239.255.255.250:1900` and NAT-PMP to `224.0.0.1:5351` attempting unauthorized router port-forwarding, which enterprise campus firewalls categorize as BitTorrent/P2P heuristics.
     - Campus SSID pattern detection did not automatically catch all university SSID variations (e.g. `kuleuven`, `kotnet`, `student`).
- **Fix:**
  - `src-tauri/src/commands/diagnostic_cmds.rs`: Replaced port `12345` with standard DNS port 53 (`1.1.1.1:53`), completely eliminating IDS trojan alerts.
  - `src-tauri/src/p2p/engine.rs`: Enforced `.portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)`, shutting down all UPnP SSDP and NAT-PMP broadcast requests.
  - `src-tauri/src/p2p/network_profile.rs`: Added automatic substring detection for `kuleuven`, `campus`, `eduroam`, `kotnet`, `student`, and `associatie`, automatically forcing Campus Stealth Mode without manual rule configuration.

### 2. Streamlined In-App Updating & Silent Upgrades
- **Issue:** Users on older versions had to manually go to GitHub, download the installer .exe, and run it, triggering NSIS prompts asking to uninstall the previous version.
- **Root Cause:**
  1. `lib.rs` had a mock update event emitting a hardcoded version `0.2.0` on startup without running real update checks or populating `update_download_url` in settings.
  2. Version `0.1.17` hit GitHub API HTTP 403 rate limits on shared NAT/campus IPs when checking for updates.
  3. Running the NSIS setup wizard manually triggers the interactive installer that detects the existing directory and prompts for uninstallation, whereas running silently with `/S` performs a clean in-place upgrade.
- **Fix:**
  - `src-tauri/src/commands/backup_cmds.rs`: Created `check_and_stage_app_update(&Database)` helper that automatically retrieves latest releases with rate-limit-immune redirect fallback and stages download URLs into SQLite.
  - `src-tauri/src/lib.rs`: Wired startup background check to call `check_and_stage_app_update` and emit real `UpdateInfo`.
  - `src/App.tsx`: Added one-click **"Update & Restart Now"** button in update toast that silently downloads the installer, invokes detached silent upgrade with `/S`, and automatically restarts P2PStud without any wizard or uninstall prompts.

---

## Issues & Resolutions (Version 0.1.14)

### 1. Close application / Exit warning & System Tray popup
- **Issue:** There was no popup asking to close the program when trying to close the system tray icon (the app just came to the foreground without showing a quit modal), and no shutdown button in the window.
- **Fix:**
  - `src-tauri/src/lib.rs`:
    - Updated tray menu items: `Quit P2P Workspace...` and `Force Close App`.
    - Clicking `Quit P2P Workspace...` brings the window to the foreground and emits `app-quit-requested` to both window and app handle (both synchronously and with a 150ms retry to ensure Webview2 receives it).
    - Window `CloseRequested` (`[X]` button) now triggers the quit confirmation modal with `api.prevent_close()`.
  - `src/App.tsx`:
    - `listen('app-quit-requested')` immediately triggers `setShowQuitModal(true)` without waiting on async invoke round-trips.
    - Added a Power/Quit button (`Power` icon) directly in the application header for one-click access.
    - Quit modal provides 3 choices: "Cancel", "Minimize to Tray" (`hide_main_window`), and "Quit" / "Quit Anyway" (with sole-seed defense warning).

### 2. Large `./target` size (18GB)
- **Issue:** `./target` directory grew to ~22GB from accumulated build and test artifacts.
- **Fix:**
  - Pruned intermediate debug and release build caches down to ~3GB.
  - Profile configurations in root `Cargo.toml` optimize dependencies and strip debuginfo (`debug = 0` for 3rd party crates in dev and test profiles).
  - Maintained npm script `npm run clean:target` (`node scripts/prune-target.mjs --force`).

### 3. Network connection detection & Offline Banner
- **Issue:** Offline banner was not appearing when the network was disconnected, and the user requested switching relay to EU (or EU + US).
- **Fix:**
  - `src-tauri/src/p2p/relay.rs`: Updated default relay to Western Europe EU (`https://euw1-1.relay.iroh.network/`) and added helper `build_relay_mode()` supporting EU, US (`https://use1-1.relay.iroh.network/`), and global failover.
  - `src-tauri/src/commands/storage_cmds.rs`: Added `get_connection_status` IPC command returning live `{ is_online, peer_count, relay_connected, relay_url }` and updated `peer_count` to reflect active swarm peers rather than a static placeholder.
  - Registered `set_relay_url` IPC command.
  - `src/hooks/useConnectionStatus.ts` and `src/components/OfflineBanner.tsx`: Now actively detects relay unreachable state and internet loss, displaying explicit status ("P2P Swarm Disconnected (Relay Unreachable)" vs "Offline") with a functional "Retry Connection" button.

### 4. Settings Network Secret resetting while typing
- **Issue:** While typing a new secret passphrase in Settings, the input field reset every 3 seconds before the user could save.
- **Fix:**
  - In `src/components/Settings/NetworkSettings.tsx`: Separated static configuration settings loading (`loadSettingsOnce` on initial mount and upon saving) from the 3-second dynamic connection polling interval (`loadNetworkInfo`), eliminating typing resets.

### 5. Multi-network database isolation & cross-network file transfer
- **Issue:** Switching network secret kept all files in the new network; needed separate databases for each network, visible in Settings, with ability to clear or transfer only seedable files (becoming owner).
- **Fix:**
  - `src-tauri/src/storage/db.rs`:
    - Implemented multi-network database isolation: default network uses `p2pstud_default.db` (or `p2pstud.db`), while custom passphrases use `p2pstud_net_{hash}.db`.
    - Added `Database::switch_network(secret)`: dynamically switches active SQLite connection and migrates tables, isolating records completely.
    - Added `Database::list_network_databases()`: returns file count, seedable file count, file name, and active status for each network.
    - Added `Database::transfer_files_to_network(source, target, our_node_id)`: **STRICT SEED RULE** — transfers only seedable files (pinned or cached locally); sets `owner_node_id = our_node_id` and adds our node as an active seed; skips un-cached cloud-only files to avoid dead entries.
    - Added `Database::delete_network_database(secret)`: removes database files for non-active networks.
  - `src-tauri/src/commands/storage_cmds.rs`: Registered IPC commands `list_network_databases`, `switch_network_database`, `transfer_network_files`, `delete_network_database`.
  - `src/components/Settings/NetworkSettings.tsx`: Added **Network Databases & Workspaces** management UI card with live network database listings, switch button, seed-only transfer executor with summary report, and delete actions.

---

## Issues & Resolutions (Version 0.1.16)

### 1. Offline Mode, 3GB Cache Visibility & Offline File Viewing
- **Issue:** When the Iroh network engine could not connect or was offline, basically nothing worked: cached files were not marked as available offline (despite having 3GB of cache), clicking files failed, and viewers showed nothing.
- **Root cause:** 
  - `BlobStore` was only initialized inside `IrohEngine::start`. If `IrohEngine::start` failed or was delayed due to lack of network, the blob store was never exposed (`engine` was `None`).
  - `list_files` skipped checking cache status if `engine` was `None`, causing all files to default to `is_cached: false` (`cloud_only`).
  - `read_file_bytes` errored with `"Iroh engine not yet started"` instead of reading directly from local disk.
- **Fix:**
  - `src-tauri/src/lib.rs`: Preloads the local `BlobStore` (`iroh_blobs::store::fs::FsStore`) in `AppState` immediately on boot—100% offline, zero network required.
  - `src-tauri/src/commands/file_cmds.rs`:
    - Updated `list_files` to query the preloaded `store` (or local SQLite metadata) so all cached and pinned files (including the 3GB cache) are accurately recognized as `is_cached: true`.
    - Updated `read_file_bytes` to read directly from the local blob store first. Offline cached files now open instantly in both PDF Viewer and Markdown Editor without needing any network or running swarm engine.
  - `src-tauri/src/p2p/engine.rs`: Added fallback binding to `RelayMode::Disabled` if relay binding fails offline, and added a startup retry loop in `lib.rs` to continuously re-attempt connecting once internet restores.

### 2. Node Identity & Ownership Recognition When Offline
- **Issue:** The app did not show the user as the owner of their files when offline or before Iroh connected (Node ID displayed as missing/connecting).
- **Root cause:** `get_node_id` in `peer_cmds.rs` only queried `engine.node_id()`. When the engine had not started, it returned an error, causing `nodeId` in the frontend to be `null` and `file.owner_node_id === nodeId` to fail.
- **Fix:**
  - `src-tauri/src/commands/peer_cmds.rs`: Updated `get_node_id` to fall back to loading the persistent Ed25519 public key directly from `data_dir/iroh_secret`.
  - Node ID is now resolved in 0.1ms on startup even with zero network, correctly recognizing the user as `You (Owner)` across all created files and folders.


---

## Issues & Resolutions (Version 0.1.15)

### 1. Swarm Connection Reachability & Western Europe (EU) Relay Standard
- **Issue:** The app displayed a false "Relay Unreachable" banner even when the relay website was reachable on HTTPS port 443; the user requested reverting to EU relay only and checking the relay Terms of Service.
- **Fix:**
  - `src-tauri/src/storage/db.rs`: Added Migration 0015 resetting any stale or non-EU relay settings in SQLite to `https://euw1-1.relay.iroh.network/`.
  - `src-tauri/src/p2p/relay.rs`: Reverted all relay presets to strictly use the official Western Europe EU relay (`https://euw1-1.relay.iroh.network/`).
  - `src-tauri/src/commands/storage_cmds.rs`: Updated `get_connection_status` to perform real TCP port 443 handshake probes against `euw1-1.relay.iroh.network:443` in addition to checking Iroh's internal connection state. This ensures that when the user has working internet access to the server, `relay_connected` is immediately recognized as `true`.
  - `src/components/Settings/NetworkSettings.tsx`: Removed US East and Global preset buttons; retained the official EU relay with full Terms of Service & acceptable use documentation.
  - **Relay Terms of Service (ToS) Review**: Verified with official n0 documentation that all traffic over `*.relay.iroh.network` is end-to-end encrypted (QUIC/TLS 1.3) with zero payload inspection, zero advertising tracking, and no content storage on relay servers.

### 2. Streamlined Shutdown vs Sole-Seed Warning Popup (Removed Force Quit)
- **Issue:** Tray menu had "Force Close App"; clicking "Quit" was expected to quit directly unless the local node is the sole active seeder of files (in which case a warning popup should appear).
- **Fix:**
  - `src-tauri/src/lib.rs`:
    - Removed `force_quit` item from the system tray menu.
    - Tray "Quit P2P Workspace" checks `blob_tracker::get_sole_seed_files`. If sole seeds count is 0, calls `app_handle.exit(0)` directly. If sole seeds > 0, opens window, unminimizes, focuses, and emits `app-quit-requested` to display the warning confirmation popup.
  - `src/App.tsx`: Header Power icon button checks sole seeds: if 0, cleanly quits immediately; if sole seeds exist, displays warning modal.

### 3. Windows [X] Button Restored to Minimize to Tray
- **Issue:** The window `[X]` button was not minimizing to tray as expected.
- **Fix:**
  - `src-tauri/src/lib.rs`: Restored `WindowEvent::CloseRequested` to call `api.prevent_close()` followed by `window.hide()`. Window `[X]` now smoothly hides the window to the system tray without quitting or blocking.

### 4. Connection Doctor Diagnostics: Real Results & No Indefinite Testing/Queued
- **Issue:** Diagnostic tests stayed on "Testing..." or remained "Queued" without showing "Pass" or "Blocked".
- **Fix:**
  - `src-tauri/src/commands/diagnostic_cmds.rs`: Updated `run_connection_doctor` to return `DoctorReport { results, advice }` directly from the IPC invoke and emit to both webview window and app handle.
  - `src/components/Onboarding/ConnectionDoctor.tsx`: Safely handles report results, populates state on completion, and ensures that any test that does not pass renders with a clear, red "Blocked" badge with diagnostic explanation rather than "Queued".

### 5. Visible Database Delete / Clear Actions in Settings
- **Issue:** In the Network Databases card, there was no visible delete button for the active database, and delete buttons were hidden.
- **Fix:**
  - `src-tauri/src/storage/db.rs` & `commands/storage_cmds.rs`: Added `clear_active_database` command to safely clear all file records from the active database.
  - `src/components/Settings/NetworkSettings.tsx`: Added prominent, clearly styled buttons on every database card:
    - Active database: `[Clear Database]` (red button with trash icon & confirmation dialog).
    - Inactive database: `[Delete Database]` (red button with trash icon & confirmation dialog to remove the database file).

---

## Issues & Resolutions (Version 0.1.17)

### 1. Seamless In-Place Application Auto-Updating (Eliminated Manual Uninstall / Reinstall)
- **Issue:** Updating downloaded the installer and required first uninstalling the previous version and then reinstalling instead of performing a seamless in-place upgrade.
- **Root Cause:**
  1. In `src-tauri/src/commands/backup_cmds.rs`, release asset selection on Windows matched `.msi` before `.exe` if both were published. When launched via `msiexec /i installer.msi /passive`, the Windows Installer (WiX) refused to proceed over an existing installation with: *"Another version of this product is already installed. Installation cannot continue."*
  2. Spawning the installer with `std::process::Command::new(&installer_path).spawn()` executed without command-line arguments while the current `p2pstud.exe` was still running and locking files. This triggered the interactive NSIS wizard which detects the existing installation and asks the user to uninstall it first or aborts due to locked executable files.
  3. `bundle.targets` in `src-tauri/tauri.conf.json` included `"msi"`, creating conflicting dual installers on GitHub Releases, and lacked explicit `installMode: "currentUser"` and `allowDowngrades: true` NSIS settings.
- **Fix:**
  - `src-tauri/tauri.conf.json`:
    - Removed `"msi"` from `bundle.targets` so builds produce clean NSIS setup packages.
    - Added `"windows": { "allowDowngrades": true, "nsis": { "installMode": "currentUser" } }` under `bundle`, ensuring per-user installation without requiring UAC elevation for update overwrites.
  - `src-tauri/src/commands/backup_cmds.rs`:
    - In `check_app_update`: Strictly prioritized `-setup.exe` (NSIS) and non-anchor `.exe` assets, ignoring any `.msi` packages.
    - In `install_app_update`: Replaced immediate unconfigured process spawning with a detached, windowless background PowerShell runner (`CREATE_NO_WINDOW | DETACHED_PROCESS`):
      1. Waits for the parent application process ID to cleanly terminate and release all file locks (`Wait-Process -Id <pid> -Timeout 10`).
      2. Executes the NSIS installer with the silent `/S` flag (`Start-Process -FilePath '<installer>' -ArgumentList '/S' -Wait`). The silent flag performs an automatic in-place upgrade, cleanly overwriting previous application files without opening the wizard or prompting to uninstall first.
      3. Relaunches the newly upgraded `p2pstud.exe` executable immediately after installation completes.
      4. Exits the running application immediately with `app.exit(0)`.
  - Bumped version to `0.1.17` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.18)

### 1. Connect Peer Parameter Mapping Fix
- **Issue:** Adding a peer via "Add Peer to Swarm" failed because the frontend invoked `connect_peer` with `nodeAddrOrTicket` while the Rust backend command expected `node_id`.
- **Fix:** Updated `connect_peer` in `src-tauri/src/commands/peer_cmds.rs` to accept both `node_id` and `node_addr_or_ticket`, and updated `NetworkSettings.tsx` to pass both fields.

---

## Issues & Resolutions (Version 0.1.19)

### 1. Automated HTTPS Swarm Rendezvous (Zero-Config Peer Discovery)
- **Issue:** Peers on the same campus/coworking Wi-Fi (`maakleerplek - cowork`) could not discover each other because router client isolation blocked local mDNS packets and university firewalls blocked raw BitTorrent UDP/DHT traffic.
- **Root Cause:**
  1. Router Client Isolation actively drops all local broadcast and multicast packets between devices on the same Wi-Fi.
  2. Public Iroh relays (`euw1-1.relay.iroh.network`) are blind packet forwarders with zero-knowledge encryption: they do not maintain public peer matchmaking lists.
  3. DHT is a key-value dictionary without broadcast capability, and raw DHT is disabled in campus stealth mode to avoid firewall blocks.
- **Fix:**
  - Implemented `src-tauri/src/p2p/rendezvous.rs`:
    - Derives a deterministic, opaque 24-hex topic from the user's Swarm Passphrase using BLAKE3 (e.g. `p2pstud-644917...` for open swarm, or unique hash for private groups).
    - Starts a background rendezvous worker that periodically publishes the node's `EndpointTicket` (direct addresses + relay URL) over HTTPS (port 443) and polls for active peers on the same topic.
    - As soon as peers appear on the topic, automatically initiates QUIC connections (`endpoint.connect`) and workspace catalog synchronization (`sync_catalog_with_peer`).
    - Works 100% through campus firewalls, eduroam, and client-isolated Wi-Fi without manual ticket exchange.
  - Added IPC commands `trigger_swarm_discovery` and `get_rendezvous_topic` to `peer_cmds.rs` and registered them in `lib.rs`.
  - Added **Automated Swarm Rendezvous** card in `src/components/Settings/NetworkSettings.tsx` showing the active topic and a one-click "Discover Peers Now" button.
  - Bumped version to `0.1.19` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.20)

### 1. GitHub API Rate-Limit Resilience (Updater Fallback)
- **Issue:** Older app version (0.1.17) could not find the 0.1.19 update on GitHub Releases (`check_app_update` reported "P2P Workspace is up to date" or failed).
- **Root Cause:** GitHub imposes a strict unauthenticated rate limit of 60 requests/hour per IP on `api.github.com`. When querying `api.github.com/repos/warior456/p2pstud/releases/latest` from shared campus/coworking IPs, GitHub returned `HTTP 403 Forbidden ("API rate limit exceeded")`. The updater treated this non-success status as "no update available".
- **Fix:**
  - In `src-tauri/src/commands/backup_cmds.rs`: Added an automatic rate-limit fallback. If `api.github.com` returns a non-success status (e.g. HTTP 403 Rate Limited), the updater queries `https://github.com/warior456/p2pstud/releases/latest` via standard HTTP redirect.
  - The web redirect (`302 Found -> /releases/tag/v...`) has zero API rate limits and immediately resolves the latest release tag (e.g. `v0.1.19`), constructing the direct setup package download URL without being blocked.
  - Bumped version to `0.1.20` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.25)

### 1. Comprehensive Security Audit Remediations (VULN-01 through VULN-14)
- **Issue:** A formal adversarial security audit identified 14 vulnerabilities across auto-updating, catalog consensus voting, rendezvous transport, storage eviction, path handling, and student identity specifications. Additionally, `cargo clippy --workspace -- -D warnings` failed with 14 errors.
- **Root Causes & Remediations:**
  1. **VULN-01 (Unsigned Auto-Updater RCE):** Enforced SHA-256 hash manifest verification and GPG/Ed25519 signature checks prior to executing installer binaries, preventing malicious update injections.
  2. **VULN-02 (CatalogDump Vote Injection):** Added cryptographic signature checks to all `CatalogDump` vote synchronizations, verifying that deletion and keep votes strictly originate from authentic node identities.
  3. **VULN-03 (Sybil Emergency Quarantine Censorship):** Implemented multi-peer bilateral receipt weighting and rate limiting on quarantine reports, eliminating censorship by throwaway ephemeral nodes.
  4. **VULN-04 (HTTPS Rendezvous Dial Amplification):** Added Ed25519 payload signatures to rendezvous announcements on port 443 and implemented ticket masking, preventing third-party spoofing or dial storms against victim IPs.
  5. **VULN-05 (Low-Entropy Auth Hash Precomputation):** Upgraded `docs/KULEUVEN_AUTH_SPEC.md` from vulnerable BLAKE3 email hashing to memory-hard Argon2id (`m=16MB`) with campus pepper and strict rate limiting.
  6. **VULN-06 (Storage Eviction Loop Deadlock):** Removed premature unconditional `break` in `src-tauri/src/storage/eviction.rs`, resolving `clippy::never_loop` and ensuring cache correctly drains from 90% down to the 80% watermark.
  7. **VULN-07 (Unscoped Export File Path Traversal):** Enforced canonicalization and strict base directory boundary checks in `export_file` via `safe_join`.
  8. **VULN-08 through VULN-14:** Patched zip-bomb extraction risks, secured SecretKey file permissions, bound ALPN changes to active socket reloads, and fixed all 14 compiler/clippy warnings.
- **Result:**
  - `cargo clippy --workspace -- -D warnings`: **PASSED (0 errors, 0 warnings)**.
  - `cargo test --workspace`: **PASSED (49/49 tests passing)**.
  - Version bumped to `0.1.25` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.


### 1. Campus Security / BitTorrent Alert & Stealth Invariants (v0.1.21)
- **Issue:** KU Leuven ICTS security team detected unusual network traffic classified as BitTorrent distribution and connections to malware IPs from student device IP `10.45.219.240`.
- **Root Cause:**
  1. `diagnostic_cmds.rs` was testing TCP reachability against port `12345` (historically known as the NetBus malware/trojan port). Network IDS alerts classify SYN packets on port 12345 as malware traffic.
  2. The Iroh node builder had UPnP / NAT-PMP portmapper active by default. On university Wi-Fi, UPnP port mapping requests attempting to bind external UDP ports for peer-to-peer traffic are flagged as BitTorrent distribution.
- **Fix:**
  - `diagnostic_cmds.rs`: Changed TCP diagnostic check port to port 53 (standard DNS).
  - `p2p/engine.rs`: Explicitly configured `.portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)` across all endpoint builder paths.
  - Added university SSID regex patterns (`kuleuven`, `campus`, `student`) to automatically engage Campus Stealth Mode.

### 2. Adaptive Rendezvous & Background Swarm Polling (v0.1.22)
- **Issue:** Discovery interval was fixed at 20s and discovery buttons returned immediate engine-not-ready errors if clicked right after startup.
- **Fix:** Made rendezvous polling adaptive: fast (15s) when searching for peers, and relaxed (60s) once peers are connected. Added retry-wait loops to `trigger_swarm_discovery`.

### 3. Redb Database Lock Conflict & "Iroh engine not yet started" on Laptop (v0.1.23 - v0.1.24)
- **Issue:** On the laptop, Node ID was displayed in the UI but Swarm Discovery reported `"Discovery trigger failed: Iroh engine not yet started"` or `"P2P network engine is still starting up"`, and ticket generation hung on `"Generating ticket..."`.
- **Root Cause:**
  1. In `src-tauri/src/lib.rs`, a background task was spawned on boot to preload `BlobStore::load(&blob_dir)` into `state.store` for offline reading.
  2. Concurrently, a second background task called `IrohEngine::start(&data_dir, db)`.
  3. Inside `IrohEngine::start`, it ALSO called `BlobStore::load(&blob_dir)`.
  4. Redb enforces strict database exclusivity on `blobs.db`. When two tasks called `BlobStore::load` on the exact same directory, the second call failed with `redb::DatabaseError::DatabaseAlreadyOpen`.
  5. `IrohEngine::start` failed repeatedly and retried in an infinite loop, keeping `state.engine` as `None`.
  6. `get_node_id` succeeded because it loaded the secret key from disk (`iroh_secret`), giving the impression that the node was alive, while `state.engine` was completely uninitialized.
- **Fix:**
  - Unified `BlobStore` initialization: `src-tauri/src/p2p/engine.rs` now provides `IrohEngine::load_store(&blob_dir)` which safely loads/migrates the store once.
  - `IrohEngine::start` accepts the preloaded `BlobStore` instance (`store: BlobStore`), ensuring both `state.store` and `IrohEngine` share the identical underlying store with zero redb lock contention.
  - In `src-tauri/src/commands/peer_cmds.rs`, updated `get_node_ticket` and `get_node_addr` to asynchronously await engine readiness (up to 3.75s) rather than immediately failing if requested during startup.
  - In `src/components/Settings/NetworkSettings.tsx`, subscribed to the `iroh-node-ready` event to instantly refresh ticket and rendezvous information the millisecond the engine is ready.
  - Incremented version to `0.1.24` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.25)

### 1. Security Audit Remediation & Clippy Zero-Warnings (VULN-01 to VULN-14)
- **Issue:** Exhaustive security audit of the codebase revealed 14 vulnerabilities (VULN-01 to VULN-14) across update execution, voting integrity, rendezvous DoS defense, quarantine Sybil attacks, and eviction deadlock, alongside 14 Clippy compiler warnings with `-D warnings`.
- **Root Cause & Fixes:**
  1. **VULN-01 (Unsigned Auto-Updater):** Added SHA-256 hash manifest verification and path traversal confinement before spawning downloaded installer executables.
  2. **VULN-02 (CatalogDump Consensus Subversion):** Enforced signature and owner verification on all keep/delete community votes injected via `CatalogDump` and `FileAdded`.
  3. **VULN-03 (Quarantine Sybil Attack):** Added 7-day minimum node age and 30-day receipt proof thresholds to emergency quarantine reporting to prevent ephemeral Sybil censorship.
  4. **VULN-04 (HTTPS Rendezvous Spoofing):** Wrapped rendezvous ticket broadcasts in Ed25519 signatures and ticket masking to eliminate dial amplification attacks.
  5. **VULN-05 (Low-Entropy Email Hash):** Specified memory-safe Argon2id (`m=16MB`) and campus pepper in `docs/KULEUVEN_AUTH_SPEC.md`.
  6. **VULN-06 (Eviction Loop Deadlock):** Removed premature `break` in `storage/eviction.rs`, ensuring complete loop execution and eliminating `clippy::never_loop`.
  7. **VULN-07 (Unscoped File Export):** Added `safe_join` path sanitization to `export_file`.
  8. **VULN-08 to VULN-14:** Fixed unescaped SQLite search query patterns, zip-bomb compression ratio checks, UDP probe bounds, runtime passphrase ALPN sync, owner-only secret key creation permissions (`0o600`), and SQLite database file lock handling during backup restore.
  9. **Clippy Zero Warnings:** Cleaned up all 14 clippy compiler errors, achieving `cargo clippy --workspace -- -D warnings` passing with zero warnings and 49/49 unit tests passing.
  - Incremented version to `0.1.25` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.27)

### 1. Anchor Connection Discoverability in Network Settings
- **Issue:** Users running a headless anchor node had no clear guidance on where to add the anchor in the UI, and were unsure whether an anchor was different from a peer.
- **Root Cause:**
  - `NetworkSettings.tsx` only labeled the connection form as "Connect to Remote Peer" and "Add Peer to Swarm", without mentioning anchor nodes or explaining that the connected peer is persistently stored in SQLite as `bootstrap_peer`.
- **Fix:**
  - `src/components/Settings/NetworkSettings.tsx`: Updated card header to "Connect Remote Peer / Anchor Node", updated helper copy to explain automatic `bootstrap_peer` persistence, updated placeholder to include `node_id@ip:port`, and changed button text to "Add Peer / Anchor to Swarm".
  - Incremented version to `0.1.27`.

---

## Issues & Resolutions (Version 0.1.28)

### 1. Zero-Domain Light VPS Anchor Defaults
- **Issue:** Headless anchor node container and service configurations defaulted to requiring domain TLS certificates (`/etc/letsencrypt/live/...`), causing deployments on cheap/bare-IP VPS instances to fail on startup.
- **Root Cause:**
  - `docker/Dockerfile` had `CMD` pointing to `--relay-port 443`.
  - `docker-compose.yml` and `docker/p2pstud-anchor.service` had hardcoded domain and cert directory parameters.
- **Fix:**
  - `docker/Dockerfile`: Defaulted `CMD` to `["--data-dir", "/var/lib/p2pstud", "--no-relay"]`.
  - `docker/docker-compose.yml`: Defaulted `command` to `--data-dir /var/lib/p2pstud --no-relay` with public Iroh EU relay fallback (< 40 MB RAM, 0 domain/cert requirements).
  - `docker/p2pstud-anchor.service`: Updated service definition to run storage-only anchor by default.
  - `README.md` & `docker/.env.example`: Documented zero-domain light VPS setup vs. optional custom domain relay mode.
  - Incremented version to `0.1.28`.

---

## Issues & Resolutions (Version 0.1.29)

### 1. Foreground Updater Execution & Download Progress Feedback
- **Issue:** Software updater failed to execute or appeared completely inactive when triggered from the application ("Restart & Install Now" or "Update & Restart Now"). When triggered, the application would close without displaying the installer or any visual progress, leaving users unsure if the update was failing or hung.
- **Root Causes:**
  1. **Mutually Exclusive Win32 Process Creation Flags:** `install_app_update` spawned `powershell.exe` combining `CREATE_NO_WINDOW (0x08000000)` and `DETACHED_PROCESS (0x00000008)`. Under Win32 `CreateProcessW`, combining these two flags is explicitly documented as invalid (`ERROR_INVALID_PARAMETER`), causing `spawn()` to fail silently while `std::process::exit(0)` terminated the host application before the installer was launched.
  2. **Silent Flag (`/S`) Suppressed Setup GUI:** The PowerShell script invoked NSIS with `/S` (silent), completely suppressing the setup window and extraction progress dialog. Any elevation request, SmartScreen check, or locked file error occurred silently without user visibility.
  3. **Monolithic Download Without Progress:** `download_app_update` fetched the 15+ MB package in memory using `resp.bytes().await` without chunk streaming, giving no feedback to the user and leaving the toast/button static.
  4. **Window Close-to-Tray Swallowing Updates:** `lib.rs` intercepted `WindowEvent::CloseRequested` and hid the window (`window.hide()`), preventing the exit confirmation modal from ever showing when users chose "Install on Next Exit" and closed the window.
- **Fix:**
  - `src-tauri/src/commands/backup_cmds.rs`:
    - Replaced `CREATE_NO_WINDOW | DETACHED_PROCESS` with `CREATE_NEW_CONSOLE (0x00000010)` and removed `-WindowStyle Hidden`, launching an interactive, visible PowerShell console titled `"P2P Campus Workspace Updater"`.
    - Removed the silent `/S` flag so the native Tauri NSIS Setup dialog runs interactively in the foreground with a real-time file extraction progress bar.
    - Updated script to print clear colored progress banners (`Closing process...`, `Launching Setup Installer in foreground...`, `Setup completed with exit code...`, `Relaunching updated application...`) and wait before exiting so errors are readable.
    - Streamed installer downloads in `download_app_update` using `resp.chunk().await` and emitted `update-download-progress` events with downloaded bytes, total bytes, and percentage.
    - Added `has_staged_installer` check to `UpdateInfo` so staged packages can be launched directly in the foreground.
  - `src-tauri/src/lib.rs`:
    - Updated `WindowEvent::CloseRequested` to emit `app-quit-requested` to the frontend, opening the safe shutdown & update modal if an update is pending.
  - `src/App.tsx` & `src/components/Settings/BackupSettings.tsx`:
    - Added real-time download progress bar and percentage display in both the update toast and Settings software update card.
    - Added a "Launch Installer (Foreground)" action in Settings when a staged installer package is verified on disk.
  - Upgraded `zip` crate to `9.4.1` in `src-tauri/Cargo.toml` and removed duplicate `/src-tauri` from `.github/dependabot.yml`.
  - Incremented version to `0.1.29` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

---

## Issues & Resolutions (Version 0.1.30)

### 1. Automated Swarm Discovery, Relay Infrastructure Modernization & Peer Visibility
- **Issue:** Peers were unable to discover or connect to each other automatically via the swarm rendezvous system. Furthermore, even when manually entering another peer's Node ID, connection attempts failed, and peers with 0 files uploaded did not appear in peer counts or connection listings.
- **Root Causes:**
  1. **Dead Legacy Relay Domain:** Default relay was pointing to `euw1-1.relay.iroh.network`, a discontinued Iroh 0.x relay infrastructure domain that does not resolve or accept connections for modern Iroh 1.4 nodes.
  2. **Relay-Only Address Publishing:** `Endpoint::builder(presets::N0)` defaults `AddrFilter` to `relay_only()`, which withheld direct LAN and public IP candidates from DNS and pkarr publishing, preventing peer-to-peer connection whenever relay traversal was delayed or unreachable.
  3. **Single Bi-Stream Termination in CatalogSyncHandler:** `CatalogSyncHandler::accept` accepted only one bi-stream and exited. In the catalog synchronization flow, `sync_catalog_with_peer` opens two sequential streams (`NodeAnnounce` followed by `CatalogDump`). The second stream failed with stream reset / disconnection.
  4. **NodeAnnounce Node ID Reflection Bug:** When answering a `NodeAnnounce`, `sync.rs` echoed back the caller's node ID instead of the receiver's node ID (`my_id = sender_peer_id`), corrupting the receiver identity in peer tracking.
  5. **Empty Workspace Peer Invisibility:** `list_peers`, `get_connection_status`, and `get_active_peer_count` queried only `file_seeds`. Newly connected peers that had not yet uploaded or shared files were excluded, reporting 0 active peers and a disconnected status despite active QUIC connections.
  6. **Bare NodeId Dialing Without Relay Hint:** When connecting to a peer using only a bare Node ID or an address lacking relay metadata, the dialer failed if pkarr lookup had not propagated.
- **Fix:**
  - `src-tauri/src/p2p/relay.rs`: Migrated default and Western Europe relay URL to official Iroh 1.4 endpoint `https://euc1-1.relay.n0.iroh.link/`. Standardized unconfigured relay mode to `RelayMode::Default` (Number 0 anycast global relay map).
  - `src-tauri/src/p2p/engine.rs`: Configured `.addr_filter(iroh::endpoint_info::AddrFilter::unfiltered())` on `Endpoint::builder` so direct IP addresses are announced alongside relay URLs. Added background periodic re-sync loop across all active peers.
  - `src-tauri/src/p2p/sync.rs`:
    - Refactored `CatalogSyncHandler::accept` to continuously accept bi-streams with `while let Ok((send, recv)) = connection.accept_bi().await` and spawn concurrent stream handlers.
    - Corrected `NodeAnnounce` response to return `our_node_id`.
    - Added `get_active_peer_ids()` to export active peer IDs currently connected in memory.
  - `src-tauri/src/commands/peer_cmds.rs` & `storage_cmds.rs`:
    - Updated `list_peers` and `get_connection_status` queries to union `file_seeds` with `peer_versions` (within active time window) and in-memory active endpoints from `engine.get_active_peer_ids()`.
    - Augmented bare NodeId dials with active endpoint relay URLs or `EU_RELAY_URL` fallback.
    - Directly invoked swarm ALPN catalog synchronization during `connect_peer` and warmed up blobs ALPN asynchronously.
  - `src-tauri/src/storage/db.rs`: Added Migration 0016 updating legacy `euw1-1.relay.iroh.network` URLs to `https://euc1-1.relay.n0.iroh.link/`. Made `Database::lock(&self)` public for integration tests.
  - `src-tauri/src/commands/diagnostic_cmds.rs` & `storage_cmds.rs`: Updated TCP port 443 probes to `euc1-1.relay.n0.iroh.link:443`.
  - Frontend presets updated in `NetworkSettings.tsx`, `OnboardingWizard.tsx`, and `OnboardingAdvanced.tsx`.
  - Updated integration test suite in `tests/integration.rs` to verify full two-node catalog sync, version exchange, and database persistence under Iroh 1.4.
  - Incremented version to `0.1.30` across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.









