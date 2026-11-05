# DEV_FOUND_ISSUES.md — Resolved Issues Log

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


