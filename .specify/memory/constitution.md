# Nomen — Constitutional Foundation

> "Networked Environment for Metadata Organisation and Enrichment" A semantic file manager with spreadsheet-style metadata editing.

* * *

## Article I — Purpose and Vision

Nomen exists to fill the gap left by the abandonment of the semantic desktop. It is a desktop file manager that treats metadata as a first-class citizen — not an afterthought panel, but the primary interface through which files are understood, organised, and discovered.

The guiding analogy is: **file manager navigation feel + spreadsheet editing feel, unified without compromise.**

Nomen must feel as fast as Finder, as navigable as Dolphin, and as editable as a well-designed spreadsheet — simultaneously, in the same view.

* * *

## Article II — Inviolable Principles

### II.1 — Standards Fidelity

Nomen reads and writes industry-standard embedded metadata: EXIF, IPTC, XMP, ID3, and OS extended attributes (xattr/EA). It never invents a proprietary metadata layer as the primary store. User-defined metadata is written to embedded XMP by default (or XMP sidecar files), with xattr as a secondary option. Metadata travels with files because it lives in files.

### II.2 — Speed is a Feature

No operation that the user perceives as "navigation" — folder traversal, row rendering, column sorting, cell selection — may block on disk I/O. All metadata is served from a local Turso index. The index is populated and kept fresh by a background ExifTool daemon process. The UI never waits for ExifTool directly.

### II.3 — Simplicity of Interaction

The interaction model has exactly two states: **row selection** (file manager mode) and **cell editing** (spreadsheet mode). Single-click enters row selection. Double-click or F2 enters cell editing on the focused cell. The leftmost column (icon/thumbnail) is navigation-only: double-click traverses folders or opens files. This model is never compromised by feature additions.

### II.4 — Write Safety

Nomen never modifies original files without creating a backup or offering explicit confirmation. ExifTool's `-preserve` and backup mechanisms are engaged by default. Bulk edits display a clear "editing N files" affordance before committing. Undo is supported for at least the last bulk operation.

### II.5 — No Lock-In

The local Turso index is a standard SQLite file. Any tool that reads SQLite can query it. Metadata written to files is in open standards (XMP, ID3, EXIF). Nomen can be uninstalled and all metadata survives in the files themselves.

### II.6 — Offline First

Nomen is a desktop application. It functions fully without internet connectivity.

### II.7 — Progressive Enhancement

Optional capabilities (Ollama AI tagging, vector similarity search) enhance Nomen when available. Their absence does not degrade core functionality. The app detects and silently disables unavailable optional services.

* * *

## Article III — Technical Constitution

### III.1 — Stack

-   **Runtime**: Electrobun (Bun + native WebView)
-   **Language**: TypeScript throughout (main process and webview)
-   **Index database**: Turso embedded (local SQLite-compatible file)
-   **Metadata engine**: ExifTool, run as a persistent `-stay_open` daemon process, communicating over stdin/stdout
-   **Grid UI**: Glide Data Grid (canvas-rendered, high-performance)
-   **Optional AI**: Ollama (local LLM, detected at runtime)

### III.2 — Process Architecture

The application has two processes:

-   **Main process** (Bun): filesystem access, ExifTool IPC, Turso queries, file operations, xattr read/write
-   **Webview process**: grid rendering, user input, RPC calls to main process only

Webview has no direct filesystem or database access. All data flows through typed RPC.

### III.3 — Metadata Pipeline

```
File on disk
  → ExifTool daemon (extract all metadata as JSON)
  → Turso index (EAV table + vector column)
  → Grid UI (virtual rows from index)
  → User edits cell
  → Turso index update (immediate, optimistic)
  → ExifTool write-back (async, queued, batched)
  → File on disk (updated)
```

Write-back failures surface as non-blocking notifications, never silent data loss.

### III.4 — Separation of Concerns in Documentation

-   **spec.md**: Product perspective — WHAT and WHY. Technology-agnostic. Audience: product thinking.
-   **plan.md**: Engineering perspective — HOW. All technical decisions live here.
-   **data-model.md**: Schema definitions, EAV structure, index design.
-   **research.md**: Library evaluations, format coverage notes, tradeoff decisions.
-   **tasks.md**: Ordered, dependency-aware implementation tasks.
-   **quickstart.md**: Key validation scenarios for human and agent review.

### III.5 — Code Generation Rules

-   Every feature traces to a user story with acceptance criteria in GIVEN/WHEN/THEN form
-   No framework or library is added without documentation in research.md
-   Every RPC endpoint has a typed contract before implementation
-   Tests accompany every module; no implementation without test scaffolding
-   Complexity is tracked: if a task exceeds 4 hours estimated, it must be split

* * *

## Article IV — Scope Boundaries

### In Scope (v1.0)

-   Folder navigation (breadcrumb, keyboard shortcuts)
-   File grid with frozen icon column, sortable/resizable metadata columns
-   EXIF, IPTC, XMP, ID3, xattr read and write via ExifTool
-   Inline cell editing with F2/double-click
-   Bulk edit via Glide drag-handle
-   Column management (add, remove, reorder, per-folder or global)
-   Local Turso index with background indexing
-   Basic file operations (rename, delete, move, copy) via context menu
-   Named view presets (column sets)

### Out of Scope (v1.0)

-   Cloud sync
-   Network/remote filesystem support
-   AI tagging (v2)
-   Vector similarity search (v2)
-   Plugin system
-   Mobile or web versions
-   Preview pane (v1.1)

* * *

## Article V — Amendment Process

Amendments to this constitution require:

1.  Explicit documentation of rationale
2.  Assessment of impact on Articles II and III
3.  Version increment and dated entry below

### Amendment Log

| Version | Date | Change | Rationale |
| --- | --- | --- | --- |
| 1.0 | 2026-03-31 | Initial constitution | Greenfield project inception |
