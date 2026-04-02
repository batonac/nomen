# Nomen — Research & Decision Log

**Version**: 1.0 | **Date**: 2026-03-31

* * *

## R1 — Runtime: Tauri vs Electrobun vs Electron

| Concern | Tauri | Electrobun | Electron |
| --- | --- | --- | --- |
| Bundle size | ~10MB | ~12MB | ~150MB+ |
| Language | Rust backend + JS frontend | TypeScript throughout | JavaScript throughout |
| ExifTool subprocess | Rust std::process::Command | Bun native (Bun.spawn) | Node child_process |
| Startup time | ~100ms | <50ms | ~500ms+ |
| Maturity | Mature (v2 stable) | Young (v1, March 2026) | Very mature |
| Cross-platform Linux | WebKitGTK (native) | WebKitGTK, bundled browser option | Chromium bundled |
| NixOS support | First-class (in nixpkgs) | Broken (segfault in bundled Bun FFI) | Works |
| Grid library ecosystem | Full JS/TS ecosystem | Full JS/TS ecosystem | Full JS/TS ecosystem |
| SQLite / Turso | Native via `turso` crate (pure Rust) | @tursodatabase/database | better-sqlite3 |

**Decision**: Tauri v2. Electrobun was initially chosen for TypeScript-throughout simplicity, but its Linux support is not production-ready — the bundled Bun binary segfaults during FFI calls to `libNativeWrapper.so` on NixOS and there is no workaround. Tauri's Rust backend is a natural fit for ExifTool subprocess management, xattr syscalls (via the `xattr` crate), and direct SQLite/libSQL access. Tauri v2 is mature, ships in nixpkgs, and uses the system WebKitGTK — no binary patching required. The cost is maintaining Rust alongside TypeScript, but the backend logic (DB, ExifTool daemon, file ops) maps cleanly to Rust.

* * *

## R2 — Grid Component: Glide Data Grid vs TanStack Table vs AG Grid

| Concern | Glide Data Grid | TanStack Table | AG Grid Community |
| --- | --- | --- | --- |
| Rendering | Canvas | DOM | DOM |
| Performance at 10k rows | Excellent | Good with virtualisation | Good with virtualisation |
| Inline editing | Built-in | DIY | Built-in |
| Custom cell types | Supported | DIY | Supported |
| Fill handle (bulk drag) | Built-in | DIY | Built-in (Community) |
| Frozen columns | Built-in | DIY | Built-in |
| Visual aesthetic | Clean, neutral | Unstyled | Enterprise-heavy |
| Clipboard handling | Custom (canvas limitation) | Browser native | Custom |
| License | MIT | MIT | MIT (Community) |

**Decision**: Glide Data Grid. Canvas rendering is critical for 60fps scrolling with thousands of rows and custom cell renderers (thumbnails, tag chips, star ratings). The built-in fill-handle for bulk editing maps directly to US-05. The main cost is custom clipboard implementation — accepted.

* * *

## R3 — Metadata Engine: ExifTool vs Rust crates (rexiv2, id3, xmp\_toolkit)

| Concern | ExifTool | Rust crates |
| --- | --- | --- |
| Format coverage | 150+ formats, 30,000+ tags | Image (rexiv2), Audio (id3), XMP (xmp_toolkit) separately |
| Write support | Comprehensive | Partial; rexiv2 depends on C++ gexiv2/Exiv2 |
| Maintenance | Actively maintained (Phil Harvey) | Mixed; jExifToolGUI abandoned 2025 |
| Distribution | Perl runtime required | Compiled into binary |
| IPC overhead (per-file spawn) | 200–500ms | None (in-process) |
| IPC overhead (stay_open daemon) | ~1ms per file | N/A |
| Long-tail format support | Unmatched | Limited |

**Decision**: ExifTool with `-stay_open` daemon mode. The per-file spawn overhead is unacceptable for a file manager but the daemon mode eliminates it. Distribution requires bundling Perl or a standalone ExifTool binary (ExifTool ships a standalone macOS binary; Linux distribution via package manager or bundled). The format coverage advantage over Rust crates is decisive — ExifTool is effectively irreplaceable for production metadata work.

**Distribution plan**: Bundle the ExifTool standalone binary in the Electrobun app package. Version-pin it. Users do not need to install Perl.

* * *

## R4 — Local Database: Turso vs plain SQLite vs Bun's built-in SQLite

| Concern | Turso | Plain SQLite (better-sqlite3) | Bun built-in SQLite |
| --- | --- | --- | --- |
| Async I/O | Native (io_uring on Linux) | Synchronous | Synchronous |
| Vector search | Native F32_BLOB | Via extension (sqlite-vec) | Not built-in |
| Concurrent writes | Multiple writers (soon) | Single writer | Single writer |
| SQLite compatibility | 100% file format | 100% | 100% |
| Turso Cloud sync | Optional | No | No |
| Maturity | Beta | Very mature | Stable, newer |
| v1.0 need for vectors | No (v2 feature) | No | No |

**Decision**: Turso embedded. The async I/O design is valuable for a file manager's background indexing workload. Vector search will be used in v2 (AI similarity features) — having it available without a future migration is worth the modest additional complexity. For v1.0, it behaves identically to plain SQLite from the application's perspective.

* * *

## R5 — xattr Approach: Bun FFI vs Node addon vs shell out

| Concern | Bun FFI | Node native addon | xattr CLI shell-out |
| --- | --- | --- | --- |
| Performance | In-process, fast | In-process, fast | Process spawn per call |
| Complexity | Low (direct syscall) | Medium (C++ bindings) | Low |
| Maintenance | Low | Medium | Low |
| Cross-platform | macOS + Linux (different ABIs) | macOS + Linux | macOS only (no Linux xattr CLI) |

**Decision**: Bun FFI with conditional macOS/Linux implementation. `getxattr`/`setxattr` are simple syscall wrappers. The macOS and Linux ABIs differ slightly (macOS has a `position` parameter; Linux does not) — handled with platform detection at build time.

* * *

## R6 — ExifTool Coverage Reference

Key metadata standards and ExifTool's support:

| Standard | File Types | Read | Write | Notes |
| --- | --- | --- | --- | --- |
| EXIF | JPEG, TIFF, HEIC, RAW, PNG | ✓ | ✓ | Camera metadata |
| IPTC/IIM | JPEG, TIFF | ✓ | ✓ | Press/news metadata |
| XMP | JPEG, PDF, video, sidecar | ✓ | ✓ | Extensible, preferred for custom fields |
| ID3v1/v2 | MP3, WAV, AIFF | ✓ | ✓ | Audio tags |
| Vorbis Comment | FLAC, OGG | ✓ | ✓ |  |
| MP4/iTunes | MP4, M4A, MOV | ✓ | ✓ |  |
| PDF metadata | PDF | ✓ | ✓ |  |
| PNG tEXt/iTXt | PNG | ✓ | ✓ |  |
| WebP | WebP | ✓ | ✓ |  |
| xattr | All (via OS) | Not ExifTool — handled by M6 | Not ExifTool |  |

* * *

## R7 — Nepomuk / Prior Art Analysis

The vision Nomen pursues was previously attempted by:

-   **Nepomuk-KDE** (2006–2014): EU-funded semantic desktop using RDF ontologies. Killed by performance (Virtuoso RDF store, heavy resource use on 2008 hardware) and architectural complexity. The ontological vision (arbitrary metadata on any file, cross-application linking) was correct; the implementation was premature.
-   **Baloo** (2014–present): Nepomuk's lightweight successor in KDE. Abandoned RDF for SQLite+Xapian. Faster, but also abandoned the rich semantic layer. Tags only; no editable arbitrary metadata; no spreadsheet UI.
-   **TagSpaces**: Proprietary sidecar JSON. Does not read/write standard embedded metadata. No spreadsheet view.
-   **DigiKam**: Full implementation for photos only. Not a general file manager.
-   **ExifToolGUI (FrankBijnen)**: Windows-only. Not a file manager.

**Nomen's differentiation**: General-purpose (not photo-only), standards-based (not proprietary), spreadsheet UI (not properties panel), modern stack.
