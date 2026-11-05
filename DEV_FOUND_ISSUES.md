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
