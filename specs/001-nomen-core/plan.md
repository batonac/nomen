# Nomen — Technical Implementation Plan

**Plan ID**: 001-nomen-core  
**Status**: Draft  
**Version**: 1.0  
**Date**: 2026-03-31  
**References**: spec.md, constitution.md

* * *

## Technical Context

| Concern | Decision |
| --- | --- |
| Runtime | Tauri v2 (Rust backend + system WebView) |
| Backend language | Rust |
| Frontend language | TypeScript (React) |
| Grid | Glide Data Grid |
| Index DB | Turso (embedded, local SQLite-compatible file) |
| Metadata engine | ExifTool (Perl, persistent daemon via -stay_open) |
| xattr | Rust `xattr` crate |
| Build/package | `cargo tauri build` (via Nix) |
| Frontend bundler | Bun (`bun build`) |
| Testing | `cargo test` (backend), `bun test` (frontend types) |
| Platform target | Linux (primary, NixOS), macOS (v1.0) |

* * *

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    Tauri v2 App                     │
│                                                     │
│  ┌─────────────────┐  tauri::invoke  ┌────────────┐ │
│  │  Rust Backend   │◄───────────────►│  WebView   │ │
│  │  (Tauri core)   │                 │ (Glide UI) │ │
│  │                 │                 └────────────┘ │
│  │  ┌───────────┐  │                                │
│  │  │  ExifTool │  │                                │
│  │  │  Daemon   │  │                                │
│  │  │ -stay_open│  │                                │
│  │  └───────────┘  │                                │
│  │                 │                                │
│  │  ┌───────────┐  │                                │
│  │  │  Turso   │  │                                │
│  │  │  (local)  │  │                                │
│  │  │          │  │                                │
│  │  └───────────┘  │                                │
│  │                 │                                │
│  │  ┌───────────┐  │                                │
│  │  │  xattr    │  │                                │
│  │  │  crate    │  │                                │
│  │  └───────────┘  │                                │
│  └─────────────────┘                                │
└─────────────────────────────────────────────────────┘
```

* * *

## Module Breakdown

### M1 — ExifTool Daemon (`src-tauri/src/exiftool/`)

ExifTool is invoked once at startup with `-stay_open True -@ -` flags. It reads commands from stdin and writes JSON responses to stdout. This avoids the per-file process spawn overhead (typically 200–500ms per invocation) that would make the tool unusable at scale.

**Key design**:

-   An `ExifToolDaemon` struct wraps a `std::process::Child`
-   Commands are queued and dispatched sequentially over stdin
-   Responses are correlated to commands via sequence IDs in the JSON output (`-echo4` flag pattern)
-   The daemon is restarted automatically if the process exits unexpectedly
-   All reads return `Result<ExifData>` via async channels — callers never block on process spawning

**ExifTool invocation pattern**:

```
stdin line: /path/to/file.jpg
stdin line: -json
stdin line: -all:all
stdin line: -execute0001
stdout: [{...json metadata...}]{ready0001}
```

**Outputs**: `ExifData` typed object per file, `WriteResult` for write operations

* * *

### M2 — Index Database (`src-tauri/src/db/`)

A local Turso database (`~/.nomen/index.db`) serves as the metadata cache. It is the sole data source for the grid UI — the UI never calls ExifTool directly.

**Schema** (see data-model.md for full definition):

-   `files` — one row per known file (path, inode, mtime, content\_hash)
-   `metadata` — EAV table (file\_id, namespace, key, value)
-   `views` — named column view presets
-   `columns` — user-defined column definitions
-   `write_queue` — pending ExifTool write-back operations

**Index maintenance**:

-   On folder navigation: query `files` for known entries; trigger background scan for new/changed files
-   Background scanner walks directory, compares mtime/inode to index, enqueues ExifTool reads for changed files
-   `notify` crate monitors open folders for real-time changes
-   Write-back queue is processed by a background worker; failures are logged to `write_errors` table

* * *

### M3 — Tauri Command Layer (`src-tauri/src/commands/` + `src/mainview/`)

All communication between Rust backend and webview uses Tauri's `#[tauri::command]` system. The frontend invokes commands via `@tauri-apps/api` `invoke()`. Events are pushed from Rust to the webview via Tauri's event system.

**Webview → Rust commands**:

-   `navigate_to(path: String) -> Vec<FileRow>` — change directory, returns initial rows
-   `get_metadata(file_id: i64) -> ExifData` — full metadata for a file
-   `write_metadata(writes: Vec<MetadataWrite>) -> WriteResult` — commit cell edits
-   `bulk_write(write: BulkWrite) -> WriteResult` — bulk edit operation
-   `file_op(op: FileOperation) -> FileOpResult` — rename, move, copy, delete
-   `get_views() -> Vec<NamedView>` — list saved views
-   `save_view(view: NamedView)` — persist a named view
-   `add_column(col: ColumnDefinition)` — add user-defined column

**Rust → Webview events (via `app.emit()`)**:

-   `index-update` — push updated rows to grid
-   `write-result` — notify of async write completion/failure
-   `index-progress` — background scan progress

* * *

### M4 — Grid UI (`src/mainview/`)

The webview renders the Glide Data Grid. The grid operates in a virtual mode — it requests rows by index range and Glide handles windowing.

**Column architecture**:

-   Column 0: frozen, non-editable, renders file icon or thumbnail + filename
    -   Double-click → dispatch `navigateTo` (folders) or `fileOp.open` (files)
    -   Single-click → row selection
-   Columns 1–N: metadata columns, editable per cell type
    -   Cell types: `text`, `number`, `date`, `rating`, `tags`, `boolean`
    -   Edit enters on double-click or F2
    -   Tab/Enter commits and advances focus
    -   Escape cancels

**Selection model**:

-   Row selection: click anywhere in the row (including metadata cells when not in edit mode)
-   Cell selection: enters edit mode, distinct visual state from row selection
-   Multi-row: Shift+click, Ctrl/Cmd+click, Ctrl/Cmd+A
-   Context menu on right-click of selected rows

**Bulk edit (Glide drag handle)**:

-   Glide's built-in fill-handle behaviour is used directly
-   On drag complete, webview dispatches `bulkWrite` RPC with affected rows and value

* * *

### M5 — Breadcrumb Navigation (`src/mainview/Breadcrumb.tsx`)

A custom breadcrumb component replaces the traditional sidebar tree.

-   Path segments rendered as clickable buttons
-   Click on segment → invoke `navigate_to(segmentPath)`
-   Click on separator between segments → popover with sibling folders (fetched via `list_directory(parentPath)` command)
-   F6 → breadcrumb becomes a plain text input; Enter invokes `navigate_to`
-   Keyboard: Left/Right arrows move between segments; Enter activates

* * *

### M6 — xattr Module (`src-tauri/src/xattr.rs`)

Uses the Rust `xattr` crate for cross-platform extended attribute access. On macOS this uses the `<sys/xattr.h>` API; on Linux the `<sys/xattr.h>` POSIX API. The crate abstracts both.

User-defined column values stored in xattr use the namespace prefix `user.nomen.` to avoid collisions.

* * *

### M7 — File Operations (`src-tauri/src/fileops.rs`)

Standard POSIX file operations wrapped in Tauri commands. All destructive operations (delete, move/overwrite) require confirmation via the frontend before execution. Move operations preserve xattr where the target filesystem supports it (detected at runtime).

* * *

## Constitutional Compliance Checks

-    **Speed**: Grid data served from Turso index, never from ExifTool directly. ExifTool daemon avoids per-file spawn. ✓
-    **Write safety**: ExifTool `-preserve` flag used; `write_queue` with failure logging; bulk edit confirmation affordance. ✓
-    **Standards fidelity**: All metadata written via ExifTool to standard embedded fields or XMP sidecar. ✓
-    **No lock-in**: Index is SQLite-compatible (Turso). Metadata lives in files. ✓
-    **Offline first**: No network dependency for any v1.0 feature. ✓
-    **Progressive enhancement**: Ollama and Turso Cloud not referenced in v1.0 plan. ✓

* * *

## Complexity Tracking

| Module | Estimated Effort | Risk |
| --- | --- | --- |
| M1 ExifTool Daemon | 3 days | Medium — IPC protocol quirks |
| M2 Turso Index + background scan | 4 days | Medium — fs.watch reliability |
| M3 RPC layer + type contracts | 2 days | Low |
| M4 Grid UI + column types | 5 days | Medium — custom cell renderers |
| M5 Breadcrumb navigation | 1 day | Low |
| M6 xattr FFI | 1 day | Low–Medium — macOS vs Linux ABI |
| M7 File operations | 2 days | Low |
| Integration + testing | 3 days | Medium |
| Total | ~21 days |  |

* * *

## Open Questions

1.  **Column persistence scope**: Should column layouts be per-folder, per-folder-pattern (e.g. all folders under `~/Photos/`), or global? Proposed: per-folder with an option to "apply to all subfolders". To be resolved before M4 implementation.
    
2.  **ExifTool write-back batching window**: How long to wait before flushing the write queue after a cell edit? Proposed: 500ms debounce for single edits; immediate flush for bulk edits. To be confirmed during M1/M2 integration.
    
3.  **Thumbnail generation**: Should Nomen generate thumbnails natively or rely on OS thumbnail cache? Proposed: use OS thumbnail cache (NSImage on macOS, GIO thumbnailer on Linux) for v1.0; Nomen-native thumbnails in v1.1.
