# Nomen — Data Model
**Version**: 1.0 | **Date**: 2026-03-31

---

## Database: `~/.nomen/index.db` (Turso embedded)

---

### Table: `files`

One row per file known to the index. The primary identifier for all metadata associations.

```sql
CREATE TABLE files (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  path         TEXT    NOT NULL UNIQUE,   -- absolute path
  filename     TEXT    NOT NULL,          -- basename
  extension    TEXT,                      -- lowercase, no dot
  size_bytes   INTEGER,
  mtime        INTEGER NOT NULL,          -- Unix timestamp (ms)
  inode        INTEGER,                   -- for change detection
  content_hash TEXT,                      -- SHA-1 of file content, populated lazily
  file_kind    TEXT,                      -- 'image' | 'audio' | 'video' | 'document' | 'folder' | 'other'
  indexed_at   INTEGER NOT NULL,          -- Unix timestamp (ms) of last ExifTool read
  thumbnail_path TEXT                     -- path to cached thumbnail, if generated
);

CREATE INDEX files_path_idx ON files(path);
CREATE INDEX files_mtime_idx ON files(mtime);
```

---

### Table: `metadata`

EAV (Entity-Attribute-Value) store for all metadata. Flexible enough to hold any tag from any standard without schema migrations.

```sql
CREATE TABLE metadata (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  namespace TEXT    NOT NULL,   -- 'EXIF' | 'IPTC' | 'XMP' | 'ID3' | 'xattr' | 'user' | 'system'
  key       TEXT    NOT NULL,   -- ExifTool tag name e.g. 'EXIF:DateTimeOriginal', 'ID3:Artist'
  value     TEXT,               -- stored as text; typed display handled in UI
  data_type TEXT    NOT NULL DEFAULT 'text',  -- 'text' | 'number' | 'date' | 'rating' | 'tags' | 'boolean'
  updated_at INTEGER NOT NULL   -- Unix timestamp (ms) of last update
);

CREATE INDEX metadata_file_id_idx ON metadata(file_id);
CREATE INDEX metadata_namespace_key_idx ON metadata(namespace, key);
CREATE UNIQUE INDEX metadata_file_ns_key_idx ON metadata(file_id, namespace, key);
```

**Namespace values**:
- `EXIF` — EXIF tags (camera make/model, GPS, exposure, etc.)
- `IPTC` — IPTC/IIM tags (keywords, caption, copyright, etc.)
- `XMP` — XMP properties (Dublin Core, XMP Basic, IPTC-XMP, etc.)
- `ID3` — ID3 tags for audio files (artist, album, track, genre, etc.)
- `xattr` — OS extended attributes
- `user` — User-defined fields stored via Nomen (written to XMP sidecar or xattr per column config)
- `system` — Filesystem metadata not from ExifTool (filename, size, mtime, kind) — denormalised copy for query convenience

---

### Table: `columns`

User-defined and saved column definitions. System columns (filename, size, mtime, kind) are hardcoded and do not appear here.

```sql
CREATE TABLE columns (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  label       TEXT    NOT NULL,          -- display name in column header
  namespace   TEXT    NOT NULL,          -- matches metadata.namespace
  key         TEXT    NOT NULL,          -- matches metadata.key
  data_type   TEXT    NOT NULL,          -- 'text' | 'number' | 'date' | 'rating' | 'tags' | 'boolean'
  write_dest  TEXT    NOT NULL DEFAULT 'embedded_xmp',  -- 'embedded_xmp' | 'xmp_sidecar' | 'xattr'
  width_px    INTEGER NOT NULL DEFAULT 160,
  is_sortable INTEGER NOT NULL DEFAULT 1,
  is_editable INTEGER NOT NULL DEFAULT 1,
  created_at  INTEGER NOT NULL
);

CREATE UNIQUE INDEX columns_ns_key_idx ON columns(namespace, key);
```

---

### Table: `views`

Named column view presets. A view is an ordered list of column IDs with per-view width overrides.

```sql
CREATE TABLE views (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT    NOT NULL UNIQUE,
  columns_json TEXT  NOT NULL,  -- JSON array of {column_id, width_px, frozen} objects
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

---

### Table: `folder_views`

Maps a folder path to a preferred view, enabling per-folder column layouts.

```sql
CREATE TABLE folder_views (
  path      TEXT    NOT NULL PRIMARY KEY,
  view_id   INTEGER NOT NULL REFERENCES views(id),
  apply_to_children INTEGER NOT NULL DEFAULT 0  -- cascade to subfolders
);
```

---

### Table: `write_queue`

Tracks pending and failed ExifTool write-back operations. Written optimistically to the index immediately; actual file write is asynchronous.

```sql
CREATE TABLE write_queue (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  namespace   TEXT    NOT NULL,
  key         TEXT    NOT NULL,
  old_value   TEXT,              -- for undo
  new_value   TEXT,
  status      TEXT    NOT NULL DEFAULT 'pending',  -- 'pending' | 'complete' | 'failed'
  error_msg   TEXT,
  created_at  INTEGER NOT NULL,
  completed_at INTEGER
);

CREATE INDEX write_queue_status_idx ON write_queue(status);
CREATE INDEX write_queue_file_id_idx ON write_queue(file_id);
```

---

## TypeScript Types

```typescript
// src/shared/types.ts

export type FileKind = 'image' | 'audio' | 'video' | 'document' | 'folder' | 'other';
export type Namespace = 'EXIF' | 'IPTC' | 'XMP' | 'ID3' | 'xattr' | 'user' | 'system';
export type DataType = 'text' | 'number' | 'date' | 'rating' | 'tags' | 'boolean';
export type WriteDest = 'embedded_xmp' | 'xmp_sidecar' | 'xattr';
export type WriteStatus = 'pending' | 'complete' | 'failed';

export interface FileRow {
  id: number;
  path: string;
  filename: string;
  extension: string | null;
  sizeBytes: number | null;
  mtime: number;
  fileKind: FileKind;
  thumbnailPath: string | null;
  metadata: Record<string, string | null>; // key = `${namespace}:${key}`
}

export interface ColumnDefinition {
  id?: number;
  label: string;
  namespace: Namespace;
  key: string;
  dataType: DataType;
  writeDest: WriteDest;
  widthPx: number;
  isSortable: boolean;
  isEditable: boolean;
}

export interface NamedView {
  id?: number;
  name: string;
  columns: Array<{
    columnId: number;
    widthPx: number;
    frozen: boolean;
  }>;
}

export interface MetadataWrite {
  fileId: number;
  namespace: Namespace;
  key: string;
  oldValue: string | null;
  newValue: string | null;
}

export interface BulkWrite {
  fileIds: number[];
  namespace: Namespace;
  key: string;
  value: string | null;
}

export interface WriteResult {
  success: boolean;
  affectedFiles: number;
  failedFiles: number;
  errors: Array<{ fileId: number; path: string; message: string }>;
}

export interface IndexProgress {
  folderPath: string;
  total: number;
  indexed: number;
  phase: 'scanning' | 'extracting' | 'complete';
}
```

---

## Key Queries

### Get all files in a folder with metadata for active columns

```sql
SELECT
  f.id, f.path, f.filename, f.extension, f.size_bytes,
  f.mtime, f.file_kind, f.thumbnail_path,
  m.namespace || ':' || m.key AS meta_key,
  m.value
FROM files f
LEFT JOIN metadata m ON m.file_id = f.id
  AND (m.namespace || ':' || m.key) IN (/* active column keys */)
WHERE f.path LIKE ? || '%'
  AND f.path NOT LIKE ? || '%/%/%'  -- direct children only
ORDER BY f.filename ASC;
```

### Get pending writes (for write-back worker)

```sql
SELECT wq.*, f.path
FROM write_queue wq
JOIN files f ON f.id = wq.file_id
WHERE wq.status = 'pending'
ORDER BY wq.created_at ASC
LIMIT 50;
```

### Undo last bulk edit

```sql
UPDATE write_queue SET status = 'pending', new_value = old_value, old_value = new_value
WHERE id IN (
  SELECT id FROM write_queue
  WHERE status = 'complete'
  ORDER BY completed_at DESC
  LIMIT /* N files in last bulk op */
);
```
