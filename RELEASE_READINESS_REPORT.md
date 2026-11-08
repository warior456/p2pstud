# p2pstud — Release Readiness Audit

**Audit date:** 2026-11-08
**Version audited:** 0.1.44
**Scope:** Full desktop application — React/Vite frontend, Tauri/Rust backend, SQLite storage layer, P2P sync, anchor daemon, packaging.

---

## 1. Executive summary

The app builds and its existing test suite passes, but it is **not release-ready**. There are several user-facing correctness defects that break core flows (large-file preview/download, note editing, offline uploads), plus data-integrity problems where remote catalog entries are treated as locally cached files. These affect storage accounting, eviction, sole-seed warnings, and the quit guard.

**Baseline verification**

| Check | Result |
|---|---|
| `npm run build` | ✅ Pass (Vite, 2096 modules) |
| `npm run test` | ✅ Pass (6 files, 37 tests) |
| `cargo test --workspace` | ✅ Pass (see backend section) |
| `cargo clippy --workspace --all-targets -- -D warnings` | ❌ **Fail — 2 errors** |

**Verdict:** Not shippable. Blockers are listed in §2; fix list in §3; lower-severity issues in §4.

---

## 2. Release blockers (must fix)

### B1. Large files (> 8 MB) cannot be previewed, opened, or downloaded
**Severity: Critical**

`read_file_bytes` returns the full buffer only when `data.len() <= 8 MB`. Above the limit it emits `file-chunk` / `file-chunk-done` events and returns `Ok(vec![])`.

- Backend: `src-tauri/src/commands/file_cmds.rs:136-377` (`INLINE_LIMIT` at line 141, chunk emit at 355-374, `Ok(vec![])` at 376).
- Frontend callers only use the return value and have **no** event listener:
  - PDF open: `src/App.tsx:1163-1174`
  - Download: `src/App.tsx:1215-1226`
  - Markdown load: `src/components/Viewers/MarkdownEditor.tsx:29-40`

**Impact:** The result for any PDF or note larger than 8 MB is an empty byte array — the PDF viewer shows nothing, download silently does nothing, and the Markdown editor keeps its placeholder content. This is the single biggest functional regression.

**Fix direction:** Add a frontend chunk-collector helper that subscribes to `file-chunk`/`file-chunk-done`, and use it for PDF/Markdown. For download, prefer a backend save-to-disk command so the file never round-trips through JS.

---

### B2. Notes / Markdown files are never loaded or saved
**Severity: Critical**

`App.tsx:1699-1704` renders `<MarkdownEditor initialContent=... />` with no `hash` and no `onSave`. Therefore:

- Selecting a `.md`/`.txt` file only switches the view (`App.tsx:1175-1177`); the real file content is never loaded.
- The editor displays a hardcoded placeholder.
- Typing shows the "Saved" indicator (`MarkdownEditor.tsx:46-60`), but nothing is persisted anywhere.

**Impact:** Core note-taking / collaborative editing flow is non-functional and misleadingly reports success.

**Fix direction:** Store the selected file hash, pass it to `MarkdownEditor`, and implement `onSave` via an upload/update command (same hash if content unchanged semantics allow, otherwise new blob + catalog update). At minimum, load by `hash` so read-only preview works, and only show “Saved” when a save actually happened.

---

### B3. Remote catalog rows are counted as locally cached files
**Severity: Critical (data integrity)**

`upsert_blob` always writes `written_at = now` on insert, including rows created by peer catalog sync:

- `upsert_blob`: `src-tauri/src/storage/blob_tracker.rs:532-601` (insert always sets `written_at`).
- Remote sync path: `src-tauri/src/p2p/sync.rs` (upserts catalog entries received from peers).
- `list_files` treats `written_at > 0` as “cached” even when the blob is absent from disk: `src-tauri/src/commands/file_cmds.rs:104-106`.

**Consequences:**

1. Cloud-only files show as ✅ cached / offline-available in the explorer.
2. `get_soil_seed_files` (`blob_tracker.rs:1238-1281`) has no local-presence check, so every catalog row without a remote seed is reported as a sole seed — producing false “you are the only seed” warnings and potentially blocking quit.
3. Storage stats inflate: `get_storage_stats` uses `used_bytes = disk_bytes.max(db_bytes)` where `db_bytes = total_bytes()` sums the **entire catalog** (`storage_cmds.rs:74-78`; `total_bytes` at `blob_tracker.rs:672-680`).
4. Eviction uses `total_unpinned_bytes()` over all catalog rows (`eviction.rs:113`, `blob_tracker.rs:661-669`), so the cache appears over budget and evicts aggressively / needlessly.

**Fix direction:** Introduce an explicit “locally present” signal. Options:
- Add a `locally_present` / cache-state column populated only by local writes and cleared on eviction/delete; remote-synced rows keep `written_at = 0`.
- Or derive presence from store/disk at query time for the small set of paths that need it.

Then make `list_files`, `get_sole_seed_files`, `get_storage_stats`, and eviction all use the same predicate.

---

## 3. High-priority fixes

### H1. Duplicate upload-bandwidth accounting
**Severity: High (telemetry incorrect)**

`record_upload(size_bytes)` is called multiple times per file:

- `add_file`: 3× — `file_cmds.rs:439, 442, 445`
- `upload_file_bytes`: 2× — `file_cmds.rs:573, 576`
- ZIP import: 2× per entry — `file_cmds.rs:1155, 1179`

Upload speed in the telemetry footer is inflated 2–3×. Fix: exactly one call per successfully stored file, after success.

### H2. Uploads fail when the P2P engine is not yet started, even though the local store exists
**Severity: High (offline usability)**

`add_file`, `upload_file_bytes`, `upload_file_path`, and ZIP import require `state.engine` and error with `"Iroh engine not yet started"`:

- `file_cmds.rs:397-399`
- `file_cmds.rs:510-512`
- `file_cmds.rs:1040+` (zip import)

But `read_file_bytes` already supports the preloaded `state.store` fallback. Uploading during startup or while offline should still write to the local store and DB.

**Fix direction:** Resolve store via `engine.store` **or** `state.store`; skip broadcasting when no engine; set `owner_node_id` from persisted identity if available.

### H3. `export_file` is broken and unused
**Severity: High**

- Ignores the requested destination path: always joins the filename under the OS Downloads dir (`file_cmds.rs:889-898`).
- Rejects legitimate names containing `..` anywhere (`file_cmds.rs:871-873`).
- Loads the whole blob into RAM (`file_cmds.rs:882-887`).
- Frontend never calls it (`export_file` occurs nowhere in `src/`).

**Fix direction:** Either remove it, or make it honor an absolute `dest_path` validated to be a real file path outside protected app dirs, stream the blob, and wire “Save As…” to it via the native dialog.

### H4. Unsafe/silent ZIP upload path
**Severity: High**

`handleExecuteUploadAsZip` in `src/App.tsx` can complete with no source path/data and no feedback. It also duplicates deletion-tombstone removal calls in backend upload paths (`file_cmds.rs:420/423`, `554/557`). Audit and harden these before release.

---

## 4. Medium / low priority (UX & polish)

| ID | Issue | Location | Severity |
|---|---|---|---|
| M1 | File extension checks are case-sensitive, so `.PDF`, `.MD`, `.TXT` are not recognized. | `App.tsx:1163, 1175` | Medium |
| M2 | Selecting an unsupported file type does nothing (no toast/message). | `App.tsx:1161-1178` | Medium |
| M3 | Telemetry fallback version is stale (`0.1.32`) and does not match the package version. | `Footer/TelemetryBar.tsx:71-74` | Low |
| M4 | Pin/star tooltip claims “offline available” even for catalog-only files. | `Explorer/FileRow.tsx` | Low |
| M5 | `written_at` no-op optimization in `upsert_blob` returns early on attribute equality, which can skip local-write marking once presence semantics change. | `blob_tracker.rs:566-594` | Medium |
| M6 | `get_seed_count` and buddy/abundance logic rely on the same conflated cache flag. | `blob_tracker.rs:501-530` | Medium |
| M7 | `cargo clippy -D warnings` fails on two trivial lints. | `peer_cmds.rs:452`, `db.rs:942` | Build gate |
| M8 | No frontend chunk streaming tests; large-file path untested. | `src/**` | Medium |
| M9 | Quit guard can block on false sole-seed list (see B3). | `lib.rs` tray quit + `get_sole_seed_files` | High (follows B3) |
| M10 | ZIP import progress emits inside the loop but upload accounting is misplaced/duplicated. | `file_cmds.rs:1154-1179` | Low |

---

## 5. Backend & build hygiene

### Build gate
- `cargo clippy --workspace --all-targets -- -D warnings`:
  - `src-tauri/src/commands/peer_cmds.rs:452` — `while let Some(_) = set.join_next().await {}` → `clippy::redundant_pattern_matching`. Use `while set.join_next().await.is_some() {}`.
  - `src-tauri/src/storage/db.rs:942` — `d.network_secret == ""` → `clippy::comparison_to_empty`. Use `d.network_secret.is_empty()`.

### Test coverage gaps
- No tests for: chunk-event path, `export_file`, upload-without-engine, catalog-vs-local cache distinction, `get_sole_seed_files` presence filtering, ZIP import bandwidth accounting.
- Existing eviction/storage/path tests pass, but they do not model remote catalog-only rows.

---

## 6. UI consistency notes

- Explorer badges and context-menu wording imply offline availability based on a flag that currently conflates catalog presence with local cache (B3). Fixing B3 also fixes the misleading UI.
- Telemetry footer version badge fallback is stale (M3).
- Empty/error states exist for PDF and markdown, but not for unsupported types or large-file failures (M2 / B1).

---

## 7. Verification plan after fixes

1. `cargo clippy --workspace --all-targets -- -D warnings` → zero warnings.
2. `cargo test --workspace` → all pass, including new regression tests for:
   - remote catalog row is **not** cached, **not** a sole seed, **not** counted in used bytes;
   - local upload is cached and counted exactly once;
   - upload works with `engine = None` and a preloaded store;
   - large-file read path.
3. `npm run build` and `npm run test` → pass.
4. Manual: open > 8 MB PDF, open/save `.md` with uppercase extension, download a large file, verify telemetry version and upload speed.

---

## 8. Recommended release sequence

1. Fix B1–B3 (blockers).
2. Fix H1–H4 (correctness/usability).
3. Fix M1–M7 (polish + build gate).
4. Bump patch version in all three manifests.
5. Re-run full verification matrix and commit.

---

*This report is the audit artifact for the 0.1.44 → 0.1.45 release cycle.*

---

## 9. Resolution status (0.1.45)

The following fixes were applied in this cycle. `cargo clippy --workspace --all-targets -- -D warnings`, `npm run build`, `npm run test`, and the Rust test suite are used as gates.

| ID | Status | Change |
|---|---|---|
| B1 | ✅ Fixed | Added `src/utils/readFileBytes.ts`, which registers `file-chunk`/`file-chunk-done` listeners and assembles the buffer. PDF open, download, and Markdown load now use it, so files > 8 MB work. |
| B2 | ✅ Fixed | `App.tsx` tracks the selected markdown `hash`/name, passes them to `MarkdownEditor`, and implements `onSave` through a new `save_note` IPC command (owner-checked, tombstones the previous revision). The editor only shows “Saved” after a successful save. |
| B3 | ✅ Fixed | Added `upsert_catalog_blob` (writes `written_at = 0`) and used it for all peer catalog sync paths. `mark_blob_local` marks presence after download. `total_bytes`, `total_unpinned_bytes`, `total_pinned_bytes`, `oldest_unpinned`, and `get_sole_seed_files` now require `written_at > 0`, so catalog-only rows are not cached, not counted, not evicted, and not sole seeds. Added regression tests. |
| H1 | ✅ Fixed | Removed duplicated `record_upload` calls in `add_file`, `upload_file_bytes`, and ZIP import. |
| H2 | ✅ Fixed | Added `resolve_upload_context` so `add_file`, `upload_file_bytes`, `upload_file_path`, `save_note`, and ZIP import work with the preloaded store + persisted identity when the engine is not running; broadcasts are skipped gracefully. |
| H3 | ✅ Fixed | `export_file` now honors an absolute `dest_path`, resolves relative paths under Downloads, accepts legitimate `..`-containing names, works without a running engine, and reports uncached files clearly. |
| H4 | ✅ Fixed | `handleExecuteUploadAsZip` now throws when no upload source exists. Removed duplicate tombstone calls. |
| M1/M2 | ✅ Fixed | Extension checks are lowercased; unsupported types show a toast instead of silently doing nothing; empty/failed PDF opens report an error. |
| M3 | ✅ Fixed | Telemetry fallback version updated to `0.1.45`. |
| M4 | ✅ Fixed | Star tooltip no longer claims offline availability. |
| M7 | ✅ Fixed | Both clippy errors resolved; full `-D warnings` run is clean. |

**Verification commands run:** `cargo clippy -p p2pstud-tauri --all-targets -- -D warnings`, `cargo test -p p2pstud-tauri`, `npm run build`, `npm run test`.

**Still recommended before a public release:** manual smoke test on a real two-node swarm (large PDF, note save across peers, offline upload, eviction under quota), and wiring a native “Save As…” flow if large downloads should avoid frontend memory. These are tracked as follow-ups rather than blockers.

