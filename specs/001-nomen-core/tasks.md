# Nomen — Implementation Tasks
**Version**: 1.0 | **Date**: 2026-03-31  
**References**: plan.md, data-model.md

Tasks are ordered by dependency. Each task includes the user stories it satisfies, estimated effort, and completion criteria.

---

## Phase 0 — Project Scaffold

### T-001 · Initialise Tauri v2 project
**Effort**: 2h | **Depends on**: — | **Satisfies**: Infrastructure

- [ ] Initialise `src-tauri/` with `cargo tauri init` (or manual Cargo.toml + tauri.conf.json)
- [ ] Configure `tauri.conf.json` with app name "Nomen", identifier `dev.nomen.app`, version `0.1.0`
- [ ] Set up `src-tauri/src/main.rs` (Rust backend entry)
- [ ] Set up `src/mainview/` (React webview entry, built with Bun)
- [ ] Verify `cargo tauri dev` launches a window and hot-reloads on change
- [ ] Set up `cargo test` with a passing smoke test and `bun test` for frontend types
- [ ] Add `.gitignore`, `README.md`, `LICENSE`
- [ ] Configure `flake.nix` with Rust toolchain, Tauri deps, and dev shell

**Done when**: `cargo tauri dev` opens a window; `cargo test` passes; `cargo tauri build` produces an artifact.

---

### T-002 · Define shared types and RPC contracts
**Effort**: 3h | **Depends on**: T-001 | **Satisfies**: M3

- [ ] Create `src/shared/types.ts` with all types from data-model.md
- [ ] Create `src/shared/rpc-types.ts` defining all RPC method signatures (both directions)
- [ ] Import and validate types compile without errors in both main and webview contexts
- [ ] Write type tests confirming key type constraints

**Done when**: Types imported in both processes with no TypeScript errors.

---

### T-003 · Initialise libSQL database
**Effort**: 3h | **Depends on**: T-001 | **Satisfies**: M2

- [ ] Add `turso` crate dependency
- [ ] Create `src-tauri/src/db/schema.rs` with all CREATE TABLE statements from data-model.md
- [ ] Create `src-tauri/src/db/mod.rs` exporting a singleton DB connection to `~/.nomen/index.db`
- [ ] Apply schema migrations on startup; handle version upgrades
- [ ] Write integration test: insert a file row, query it back, delete it

**Done when**: Schema created on first run; queries return correct results.

---

## Phase 1 — ExifTool Daemon (M1)

### T-004 · Implement ExifTool daemon process management
**Effort**: 4h | **Depends on**: T-001 | **Satisfies**: US-07

- [ ] Create `src-tauri/src/exiftool/daemon.rs` — `ExifToolDaemon` struct
- [ ] Spawn ExifTool with `-stay_open True -@ -` flags via `std::process::Command`
- [ ] Implement command queue: each call returns via a `tokio::sync::oneshot` resolved when `{ready####}` token received
- [ ] Implement sequence ID tracking (`-execute####` / `{ready####}` correlation)
- [ ] Implement auto-restart on process exit with exponential backoff
- [ ] Write test: send 100 sequential file queries, verify all resolve correctly
- [ ] Write test: kill the ExifTool process mid-queue, verify restart and queue resume

**Done when**: 100 queued commands resolve correctly; crash recovery verified.

---

### T-005 · Implement ExifTool JSON extraction
**Effort**: 3h | **Depends on**: T-004 | **Satisfies**: US-07

- [ ] Create `src-tauri/src/exiftool/extract.rs` — `extract_metadata(path: &str) -> Result<ExifData>`
- [ ] Issue `-json -all:all -struct` command sequence for a given path
- [ ] Parse JSON response into typed `ExifData` struct via serde
- [ ] Handle binary/non-printable values gracefully (base64 or omit)
- [ ] Write test with known fixture files: verify EXIF, IPTC, XMP, ID3 fields extracted correctly

**Done when**: Known fixture files return expected metadata values.

---

### T-006 · Implement ExifTool write-back
**Effort**: 4h | **Depends on**: T-004 | **Satisfies**: US-04, US-05

- [ ] Create `src-tauri/src/exiftool/write.rs` — `write_metadata(path: &str, writes: &[MetadataWrite]) -> Result<WriteResult>`
- [ ] Build ExifTool command sequence with `-overwrite_original_in_place` and `-preserve`
- [ ] Handle namespace-qualified tag names (e.g. `EXIF:DateTimeOriginal`)
- [ ] Parse success/error from ExifTool stdout
- [ ] Write test: write a tag to a copy of a fixture file; verify tag appears in re-extraction

**Done when**: Written tags are readable back via ExifTool on the modified file.

---

## Phase 2 — Index & Background Scanner (M2)

### T-007 · Implement file indexer
**Effort**: 4h | **Depends on**: T-003, T-005 | **Satisfies**: US-07

- [ ] Create `src-tauri/src/db/indexer.rs` — `index_folder(path: &str)` with progress events emitted via Tauri app handle
- [ ] Walk directory (non-recursive, direct children only)
- [ ] For each file: compare mtime/inode to `files` table; skip if unchanged
- [ ] Queue ExifTool extraction for new/changed files
- [ ] Upsert `files` and `metadata` rows from ExifTool output
- [ ] Emit `IndexProgress` events during scan
- [ ] Write test: index a fixture folder of 50 files; verify all appear in `files` table

**Done when**: Fixture folder fully indexed with correct metadata in database.

---

### T-008 · Implement filesystem watcher
**Effort**: 3h | **Depends on**: T-007 | **Satisfies**: US-07

- [ ] Use `notify` crate to watch the current folder
- [ ] On file create/modify: queue re-index for that file
- [ ] On file delete: remove from `files` and cascade-delete `metadata`
- [ ] On file rename: update `path` and `filename` in `files`
- [ ] Write test: create/modify/delete files in a watched folder; verify index updates

**Done when**: Index reflects filesystem changes within 1 second of occurrence.

---

### T-009 · Implement write-back queue worker
**Effort**: 3h | **Depends on**: T-003, T-006 | **Satisfies**: US-04, US-05

- [ ] Create `src-tauri/src/db/write_worker.rs` — polls `write_queue` for `pending` rows
- [ ] Batch writes by file: collect all pending writes for a file, issue single ExifTool call
- [ ] On success: mark rows `complete`, update `metadata` table
- [ ] On failure: mark rows `failed`, store `error_msg`
- [ ] Debounce single-file writes 500ms; flush immediately for bulk writes
- [ ] Write test: enqueue 20 writes across 5 files; verify batching into 5 ExifTool calls

**Done when**: Write queue drains correctly with proper batching and error recording.

---

## Phase 3 — RPC Layer (M3)

### T-010 · Implement Tauri command handlers
**Effort**: 4h | **Depends on**: T-007, T-009, T-002 | **Satisfies**: All user stories

- [ ] Register all Tauri commands in `src-tauri/src/commands/mod.rs`
- [ ] `navigate_to`: query `files` from index for given path; trigger background scan; return `Vec<FileRow>`
- [ ] `get_metadata`: return full `metadata` rows for a file ID
- [ ] `write_metadata`: optimistic index update + enqueue to `write_queue`; return `WriteResult`
- [ ] `bulk_write`: same as `write_metadata` for N files atomically
- [ ] `file_op`: delegate to M7 file operations module
- [ ] `get_views` / `save_view` / `add_column`: CRUD against `views` / `columns` tables
- [ ] Write integration test for each command

**Done when**: All Tauri commands return correctly typed responses.

---

### T-011 · Implement Rust→Webview push events
**Effort**: 2h | **Depends on**: T-010 | **Satisfies**: US-07, US-04

- [ ] Emit `index-update` event when background scanner completes a batch
- [ ] Emit `write-result` event when write-back worker completes or fails
- [ ] Emit `index-progress` event during active scans
- [ ] Write test: verify webview receives all three event types after index updates

**Done when**: Webview receives all three push event types correctly.

---

## Phase 4 — Grid UI (M4)

### T-012 · Implement base grid shell
**Effort**: 4h | **Depends on**: T-010, T-011 | **Satisfies**: US-02

- [ ] Add `@glideapps/glide-data-grid` dependency
- [ ] Create `src/views/mainview/Grid.tsx` — Glide DataEditor component
- [ ] Wire `navigate_to` Tauri command on mount; populate grid from response
- [ ] Implement virtual row provider — request rows by index range from local state
- [ ] Implement column 0 (icon column): frozen, non-editable, renders file kind icon
- [ ] Verify 10,000-row render at 60fps (use synthetic data fixture)

**Done when**: Grid renders 10,000 rows at 60fps with correct column 0 icons.

---

### T-013 · Implement column 0 navigation behaviour
**Effort**: 3h | **Depends on**: T-012 | **Satisfies**: US-01, US-03

- [ ] Double-click on folder row → invoke `navigate_to` Tauri command
- [ ] Double-click on file row → invoke `file_op` Tauri command (OS default app)
- [ ] Single-click on any row → row selection state
- [ ] Keyboard: Enter on selected folder row → navigate in; Backspace → navigate up
- [ ] Write test: simulate click/keyboard events; verify correct RPC calls

**Done when**: Navigation and file opening work via both mouse and keyboard.

---

### T-014 · Implement row and cell selection model
**Effort**: 3h | **Depends on**: T-012 | **Satisfies**: US-03, US-04

- [ ] Single-click any cell → row selection (not cell edit mode)
- [ ] Double-click or F2 on non-icon cell → cell edit mode
- [ ] Shift+click → range selection
- [ ] Cmd/Ctrl+click → additive selection
- [ ] Cmd/Ctrl+A → select all
- [ ] Escape → exit edit mode, restore value
- [ ] Tab/Enter → commit edit, advance focus

**Done when**: All selection/edit mode transitions work correctly.

---

### T-015 · Implement custom cell renderers
**Effort**: 5h | **Depends on**: T-014 | **Satisfies**: US-04

- [ ] `TextCell` — plain editable text (default)
- [ ] `NumberCell` — numeric input with validation
- [ ] `DateCell` — date display; date picker on edit
- [ ] `RatingCell` — 5-star widget; click star or type 1–5 to edit
- [ ] `TagsCell` — chip list display; chip editor on edit (add/remove chips)
- [ ] `BooleanCell` — checkbox display and edit
- [ ] Write visual regression test for each cell type (render + edit state)

**Done when**: All six cell types render correctly and enter/exit edit mode cleanly.

---

### T-016 · Implement bulk edit fill-handle
**Effort**: 2h | **Depends on**: T-015 | **Satisfies**: US-05

- [ ] Enable Glide fill-handle feature
- [ ] On fill-handle drag complete: collect affected file IDs and value
- [ ] Display "Editing N files" confirmation toast before dispatch
- [ ] Dispatch `bulkWrite` RPC
- [ ] On completion: update grid rows from `writeResult` push

**Done when**: Dragging fill-handle over 10 rows applies value to all 10 files correctly.

---

### T-017 · Implement column management UI
**Effort**: 4h | **Depends on**: T-015 | **Satisfies**: US-06

- [ ] "+" button at right end of column header row → column add popover
- [ ] Popover fields: label, namespace (dropdown), key (autocomplete), data type, write destination
- [ ] ExifTool tag autocomplete: pre-load a JSON list of common ExifTool tag names by namespace
- [ ] Right-click column header → context menu: Hide, Freeze, Configure, Remove
- [ ] Column reorder via drag (Glide built-in)
- [ ] Column resize via drag (Glide built-in)
- [ ] "Save as view" button → name input → persist to `views` table
- [ ] View switcher dropdown in toolbar → restore saved view

**Done when**: User can add, reorder, resize, hide, and save columns; saved views restore correctly.

---

## Phase 5 — Navigation Chrome (M5)

### T-018 · Implement breadcrumb bar
**Effort**: 4h | **Depends on**: T-013 | **Satisfies**: US-01

- [ ] Create `src/views/mainview/Breadcrumb.tsx`
- [ ] Render path segments as buttons; current folder styled differently
- [ ] Click segment → `navigateTo(segmentPath)`
- [ ] Click separator → popover with sibling folders (via `listDirectory` RPC)
- [ ] F6 → text input mode; Enter → `navigateTo`; Escape → restore segment view
- [ ] Keyboard: Left/Right arrows move between segments; Enter activates

**Done when**: All breadcrumb interactions work via mouse and keyboard.

---

## Phase 6 — xattr & File Operations (M6, M7)

### T-019 · Implement xattr FFI module
**Effort**: 4h | **Depends on**: T-001 | **Satisfies**: US-08

- [ ] Create `src/bun/xattr/index.ts` with platform-conditional FFI implementation
- [ ] macOS: `getxattr(path, name, buf, size, 0, 0)` / `setxattr(path, name, buf, size, 0, 0)`
- [ ] Linux: `getxattr(path, name, buf, size)` / `setxattr(path, name, buf, size, 0)`
- [ ] Namespace all user keys as `user.nomen.${key}`
- [ ] Write test: set and get a custom xattr on a temp file (macOS and Linux)

**Done when**: xattr round-trip test passes on both platforms.

---

### T-020 · Implement file operations
**Effort**: 4h | **Depends on**: T-001 | **Satisfies**: US-09

- [ ] Create `src/bun/fileops/index.ts` with typed `FileOperation` handler
- [ ] `open`: `Bun.spawn(['open', path])` (macOS) / `xdg-open` (Linux)
- [ ] `rename`: `fs.rename` + update index
- [ ] `move`: `fs.rename` or copy+delete across volumes + update index + xattr preservation check
- [ ] `copy`: recursive copy + update index
- [ ] `delete`: `fs.unlink` / `fs.rmdir` + remove from index
- [ ] `revealInOSFileManager`: `open -R path` (macOS) / `nautilus --select path` (Linux)
- [ ] All destructive operations require confirmation token from RPC caller
- [ ] Write tests for rename, copy, delete on temp files

**Done when**: All file operations complete correctly with index updates.

---

## Phase 7 — Integration & Polish

### T-021 · End-to-end integration test suite
**Effort**: 4h | **Depends on**: T-001–T-020 | **Satisfies**: All

- [ ] Test: navigate to fixture folder → rows appear in grid
- [ ] Test: edit a cell → file metadata updated on disk
- [ ] Test: bulk edit 10 files → all 10 updated on disk
- [ ] Test: add custom xattr column → value persists after app restart
- [ ] Test: ExifTool daemon crash → auto-restart → pending writes complete
- [ ] Test: navigate into subfolder and back → breadcrumb updates correctly

---

### T-022 · Performance validation
**Effort**: 2h | **Depends on**: T-021 | **Satisfies**: Non-functional requirements

- [ ] Generate fixture folder with 10,000 synthetic files
- [ ] Measure: cold navigation to folder → time to first row render (target: <500ms from index)
- [ ] Measure: cell edit commit → index update (target: <16ms)
- [ ] Measure: single-file ExifTool write-back (target: <2s)
- [ ] Measure: 100-file bulk write-back (target: <10s)
- [ ] Measure: app cold start to interactive (target: <1s)
- [ ] Document results; flag any targets missed for optimisation

---

### T-023 · Build and distribution packaging
**Effort**: 3h | **Depends on**: T-022 | **Satisfies**: Infrastructure

- [ ] Bundle ExifTool standalone binary in app package (macOS arm64 + x86_64, Linux x86_64)
- [ ] Verify ExifTool binary path resolution at runtime
- [ ] Configure `bunx electrobun build` for macOS (code-signed, notarised)
- [ ] Configure Linux build (AppImage or .deb)
- [ ] Verify app bundle size
- [ ] Smoke-test on clean macOS machine (no Homebrew, no Perl)
