use turso::{Builder, Connection, Database as TursoDb};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub mod indexer;
pub mod watcher;
pub mod write_worker;

const SCHEMA_VERSION: i64 = 2;

/// Minimal representation of an indexed file used for change detection.
pub struct IndexedFile {
    pub mtime: i64,
    pub inode: Option<i64>,
}

/// A full file row for the grid.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileRow {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub extension: Option<String>,
    pub size_bytes: Option<i64>,
    pub mtime: i64,
    pub file_kind: String,
    pub thumbnail_path: Option<String>,
    pub metadata: HashMap<String, Option<String>>,
}

/// A single metadata row.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetadataRow {
    pub id: i64,
    pub file_id: i64,
    pub namespace: String,
    pub key: String,
    pub value: Option<String>,
    pub data_type: String,
    pub updated_at: i64,
}

/// A column definition row.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ColumnRow {
    pub id: i64,
    pub label: String,
    pub namespace: String,
    pub key: String,
    pub data_type: String,
    pub write_dest: String,
    pub width_px: i64,
    pub is_sortable: bool,
    pub is_editable: bool,
    pub created_at: i64,
}

/// A named view row.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ViewRow {
    pub id: i64,
    pub name: String,
    pub columns_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A pool of read connections backed by a shared `TursoDb` handle.
///
/// WAL mode lets any number of readers run concurrently alongside the single
/// writer, so read operations never need to acquire the write mutex.
pub struct ReadPool {
    db: TursoDb,
}

impl ReadPool {
    /// Obtain a fresh read-only connection from the pool.
    pub fn reader(&self) -> turso::Result<Reader> {
        Ok(Reader(self.db.connect()?))
    }
}

/// A short-lived read handle wrapping a single `Connection`.
pub struct Reader(Connection);

impl Reader {
    pub async fn get_files_for_folder(&self, folder_path: &str) -> turso::Result<Vec<FileRow>> {
        let prefix = if folder_path.ends_with('/') {
            folder_path.to_string()
        } else {
            format!("{folder_path}/")
        };

        let mut file_rows: Vec<FileRow> = Vec::new();
        let mut rows = self.0.query(
            "SELECT id, path, filename, extension, size_bytes, mtime, file_kind, thumbnail_path
             FROM files
             WHERE path LIKE ?1 || '%'
               AND path NOT LIKE ?1 || '%/%'
             ORDER BY
               CASE file_kind WHEN 'folder' THEN 0 ELSE 1 END,
               LOWER(filename)",
            [prefix.as_str()],
        ).await?;

        while let Some(row) = rows.next().await? {
            let id = row.get_value(0)?.as_integer().copied().unwrap_or(0);
            let path = row.get_value(1)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string();
            let filename = row.get_value(2)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string();
            let extension = row.get_value(3)?.as_text().map(|s| s.clone());
            let size_bytes = row.get_value(4)?.as_integer().copied();
            let mtime = row.get_value(5)?.as_integer().copied().unwrap_or(0);
            let file_kind = row.get_value(6)?.as_text().map(|s| s.as_str()).unwrap_or("other").to_string();
            let thumbnail_path = row.get_value(7)?.as_text().map(|s| s.clone());
            file_rows.push(FileRow { id, path, filename, extension, size_bytes, mtime, file_kind, thumbnail_path, metadata: HashMap::new() });
        }

        for fr in &mut file_rows {
            let id_s = fr.id.to_string();
            let mut meta_rows = self.0.query(
                "SELECT namespace, key, value FROM metadata WHERE file_id = ?1",
                [id_s.as_str()],
            ).await?;
            while let Some(row) = meta_rows.next().await? {
                let ns = row.get_value(0)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string();
                let key = row.get_value(1)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string();
                let val = row.get_value(2)?.as_text().map(|s| s.clone());
                fr.metadata.insert(format!("{ns}:{key}"), val);
            }
        }

        Ok(file_rows)
    }

    pub async fn get_metadata_for_file(&self, file_id: i64) -> turso::Result<Vec<MetadataRow>> {
        let id_s = file_id.to_string();
        let mut out = Vec::new();
        let mut rows = self.0.query(
            "SELECT id, file_id, namespace, key, value, data_type, updated_at
             FROM metadata WHERE file_id = ?1",
            [id_s.as_str()],
        ).await?;
        while let Some(row) = rows.next().await? {
            out.push(MetadataRow {
                id: row.get_value(0)?.as_integer().copied().unwrap_or(0),
                file_id: row.get_value(1)?.as_integer().copied().unwrap_or(0),
                namespace: row.get_value(2)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                key: row.get_value(3)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                value: row.get_value(4)?.as_text().map(|s| s.clone()),
                data_type: row.get_value(5)?.as_text().map(|s| s.as_str()).unwrap_or("text").to_string(),
                updated_at: row.get_value(6)?.as_integer().copied().unwrap_or(0),
            });
        }
        Ok(out)
    }

    pub async fn get_views(&self) -> turso::Result<Vec<ViewRow>> {
        let mut out = Vec::new();
        let mut rows = self.0.query(
            "SELECT id, name, columns_json, created_at, updated_at FROM views ORDER BY name",
            (),
        ).await?;
        while let Some(row) = rows.next().await? {
            out.push(ViewRow {
                id: row.get_value(0)?.as_integer().copied().unwrap_or(0),
                name: row.get_value(1)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                columns_json: row.get_value(2)?.as_text().map(|s| s.as_str()).unwrap_or("[]").to_string(),
                created_at: row.get_value(3)?.as_integer().copied().unwrap_or(0),
                updated_at: row.get_value(4)?.as_integer().copied().unwrap_or(0),
            });
        }
        Ok(out)
    }

    pub async fn get_columns(&self) -> turso::Result<Vec<ColumnRow>> {
        let mut out = Vec::new();
        let mut rows = self.0.query(
            "SELECT id, label, namespace, key, data_type, write_dest, width_px, is_sortable, is_editable, created_at FROM columns ORDER BY id",
            (),
        ).await?;
        while let Some(row) = rows.next().await? {
            out.push(ColumnRow {
                id: row.get_value(0)?.as_integer().copied().unwrap_or(0),
                label: row.get_value(1)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                namespace: row.get_value(2)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                key: row.get_value(3)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                data_type: row.get_value(4)?.as_text().map(|s| s.as_str()).unwrap_or("text").to_string(),
                write_dest: row.get_value(5)?.as_text().map(|s| s.as_str()).unwrap_or("embedded_xmp").to_string(),
                width_px: row.get_value(6)?.as_integer().copied().unwrap_or(160),
                is_sortable: row.get_value(7)?.as_integer().copied().unwrap_or(1) != 0,
                is_editable: row.get_value(8)?.as_integer().copied().unwrap_or(1) != 0,
                created_at: row.get_value(9)?.as_integer().copied().unwrap_or(0),
            });
        }
        Ok(out)
    }
}

pub struct Database {
    pub conn: Connection,
    #[allow(dead_code)]
    path: PathBuf,
}

impl Database {
    pub async fn open_default() -> turso::Result<(Self, ReadPool)> {
        let base = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".nomen");
        fs::create_dir_all(&base).expect("Cannot create ~/.nomen directory");
        let path = base.join("index.db");
        Self::open(&path).await
    }

    pub async fn open(path: &PathBuf) -> turso::Result<(Self, ReadPool)> {
        let db: TursoDb = Builder::new_local(path.to_str().expect("Invalid path"))
            .build()
            .await?;
        let conn = db.connect()?;

        // WAL mode: set once at the database level — all subsequent connections
        // (including those from ReadPool) automatically use WAL, giving concurrent
        // reads alongside the single write connection with no reader/writer blocking.
        let mut rows = conn.query("PRAGMA journal_mode = WAL", ()).await?;
        while rows.next().await?.is_some() {}

        let mut rows = conn.query("PRAGMA foreign_keys = ON", ()).await?;
        while rows.next().await?.is_some() {}

        let database = Database { conn, path: path.clone() };
        database.migrate().await?;

        // ReadPool keeps its own clone of the TursoDb handle so it can open
        // independent connections without going through the write mutex.
        let read_pool = ReadPool { db };

        Ok((database, read_pool))
    }

    async fn migrate(&self) -> turso::Result<()> {
        let mut rows = self
            .conn
            .query("PRAGMA user_version", ())
            .await?;
        let version: i64 = match rows.next().await? {
            Some(row) => row.get_value(0)?.as_integer().copied().unwrap_or(0),
            None => 0,
        };

        if version < SCHEMA_VERSION {
            for stmt in INITIAL_SCHEMA.split(';') {
                let trimmed = stmt.trim();
                if !trimmed.is_empty() {
                    self.conn.execute(trimmed, ()).await?;
                }
            }
            self.conn
                .execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION), ())
                .await?;
        }

        Ok(())
    }

    // ─── File helpers ────────────────────────────────────────────────────────

    /// Return the existing indexed entry for `path`, if any.
    pub async fn get_file_by_path(&self, path: &str) -> turso::Result<Option<IndexedFile>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, mtime, inode FROM files WHERE path = ?1",
                [path],
            )
            .await?;
        match rows.next().await? {
            None => Ok(None),
            Some(row) => {
                let mtime = row.get_value(1)?.as_integer().copied().unwrap_or(0);
                let inode = row.get_value(2)?.as_integer().copied();
                Ok(Some(IndexedFile { mtime, inode }))
            }
        }
    }

    /// Upsert a file row and return its `id`.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_file(
        &self,
        path: &str,
        filename: &str,
        extension: Option<&str>,
        size_bytes: Option<i64>,
        mtime: i64,
        inode: Option<i64>,
        file_kind: &str,
        indexed_at: i64,
    ) -> turso::Result<i64> {
        let ext = extension.unwrap_or("");
        let size = size_bytes.map(|s| s.to_string()).unwrap_or_default();
        let inode_s = inode.map(|i| i.to_string()).unwrap_or_default();
        let mtime_s = mtime.to_string();
        let indexed_at_s = indexed_at.to_string();

        self.conn
            .execute(
                "INSERT INTO files (path, filename, extension, size_bytes, mtime, inode, file_kind, indexed_at)
                 VALUES (?1, ?2, NULLIF(?3,''), CAST(NULLIF(?4,'') AS INTEGER), ?5, CAST(NULLIF(?6,'') AS INTEGER), ?7, ?8)
                 ON CONFLICT(path) DO UPDATE SET
                   filename   = excluded.filename,
                   extension  = excluded.extension,
                   size_bytes = excluded.size_bytes,
                   mtime      = excluded.mtime,
                   inode      = excluded.inode,
                   file_kind  = excluded.file_kind,
                   indexed_at = excluded.indexed_at",
                [path, filename, ext, size.as_str(), mtime_s.as_str(), inode_s.as_str(), file_kind, indexed_at_s.as_str()],
            )
            .await?;

        let mut rows = self
            .conn
            .query("SELECT id FROM files WHERE path = ?1", [path])
            .await?;
        let row = rows.next().await?.expect("just inserted");
        Ok(row.get_value(0)?.as_integer().copied().unwrap_or(0))
    }

    /// Upsert a metadata row for `(file_id, namespace, key)`.
    pub async fn upsert_metadata(
        &self,
        file_id: i64,
        namespace: &str,
        key: &str,
        value: Option<&str>,
        updated_at: i64,
    ) -> turso::Result<()> {
        let val = value.unwrap_or("");
        let file_id_s = file_id.to_string();
        let updated_at_s = updated_at.to_string();
        self.conn
            .execute(
                "INSERT INTO metadata (file_id, namespace, key, value, updated_at)
                 VALUES (?1, ?2, ?3, NULLIF(?4,''), ?5)
                 ON CONFLICT(file_id, namespace, key) DO UPDATE SET
                   value      = excluded.value,
                   updated_at = excluded.updated_at",
                [file_id_s.as_str(), namespace, key, val, updated_at_s.as_str()],
            )
            .await
            .map(|_| ())
    }

    /// Remove a file (and its cascaded metadata) from the index.
    pub async fn delete_file_by_path(&self, path: &str) -> turso::Result<()> {
        self.conn
            .execute("DELETE FROM files WHERE path = ?1", [path])
            .await
            .map(|_| ())
    }

    /// Update path and filename after a rename.
    pub async fn rename_file(&self, old_path: &str, new_path: &str, new_filename: &str) -> turso::Result<()> {
        self.conn
            .execute(
                "UPDATE files SET path = ?1, filename = ?2 WHERE path = ?3",
                [new_path, new_filename, old_path],
            )
            .await
            .map(|_| ())
    }

    // ─── Write-queue helpers ─────────────────────────────────────────────────

    /// Enqueue a metadata write.
    pub async fn enqueue_write(
        &self,
        file_id: i64,
        namespace: &str,
        key: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        created_at: i64,
    ) -> turso::Result<()> {
        let old = old_value.unwrap_or("");
        let new = new_value.unwrap_or("");
        let file_id_s = file_id.to_string();
        let created_at_s = created_at.to_string();
        self.conn
            .execute(
                "INSERT INTO write_queue (file_id, namespace, key, old_value, new_value, status, created_at)
                 VALUES (?1, ?2, ?3, NULLIF(?4,''), NULLIF(?5,''), 'pending', ?6)",
                [file_id_s.as_str(), namespace, key, old, new, created_at_s.as_str()],
            )
            .await
            .map(|_| ())
    }

    /// Return all pending write-queue rows with their file paths.
    pub async fn get_pending_writes(
        &self,
    ) -> turso::Result<Vec<crate::db::write_worker::PendingRow>> {
        let mut out = Vec::new();
        let mut rows = self
            .conn
            .query(
                "SELECT wq.id, wq.file_id, f.path, wq.namespace, wq.key, wq.new_value
                 FROM write_queue wq
                 JOIN files f ON f.id = wq.file_id
                 WHERE wq.status = 'pending'
                 ORDER BY wq.created_at",
                (),
            )
            .await?;
        while let Some(row) = rows.next().await? {
            out.push(crate::db::write_worker::PendingRow {
                id: row.get_value(0)?.as_integer().copied().unwrap_or(0),
                file_id: row.get_value(1)?.as_integer().copied().unwrap_or(0),
                file_path: row.get_value(2)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                namespace: row.get_value(3)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                key: row.get_value(4)?.as_text().map(|s| s.as_str()).unwrap_or("").to_string(),
                new_value: row.get_value(5)?.as_text().map(|s| s.clone()),
            });
        }
        Ok(out)
    }

    pub async fn mark_write_complete(&self, id: i64, completed_at: i64) -> turso::Result<()> {
        let completed_at_s = completed_at.to_string();
        let id_s = id.to_string();
        self.conn
            .execute(
                "UPDATE write_queue SET status = 'complete', completed_at = ?1 WHERE id = ?2",
                [completed_at_s.as_str(), id_s.as_str()],
            )
            .await
            .map(|_| ())
    }

    pub async fn mark_write_failed(&self, id: i64, error_msg: &str, completed_at: i64) -> turso::Result<()> {
        let completed_at_s = completed_at.to_string();
        let id_s = id.to_string();
        self.conn
            .execute(
                "UPDATE write_queue SET status = 'failed', error_msg = ?1, completed_at = ?2 WHERE id = ?3",
                [error_msg, completed_at_s.as_str(), id_s.as_str()],
            )
            .await
            .map(|_| ())
    }

    // ─── Column / view helpers ───────────────────────────────────────────────

    pub async fn add_column(
        &self,
        label: &str,
        namespace: &str,
        key: &str,
        data_type: &str,
        write_dest: &str,
        width_px: i64,
        created_at: i64,
    ) -> turso::Result<()> {
        let width_px_s = width_px.to_string();
        let created_at_s = created_at.to_string();
        self.conn
            .execute(
                "INSERT INTO columns (label, namespace, key, data_type, write_dest, width_px, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(namespace, key) DO UPDATE SET
                   label      = excluded.label,
                   data_type  = excluded.data_type,
                   write_dest = excluded.write_dest,
                   width_px   = excluded.width_px",
                [label, namespace, key, data_type, write_dest, width_px_s.as_str(), created_at_s.as_str()],
            )
            .await
            .map(|_| ())
    }


    pub async fn save_view(&self, name: &str, columns_json: &str, now_ms: i64) -> turso::Result<()> {
        let now_s = now_ms.to_string();
        self.conn
            .execute(
                "INSERT INTO views (name, columns_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(name) DO UPDATE SET
                   columns_json = excluded.columns_json,
                   updated_at   = excluded.updated_at",
                [name, columns_json, now_s.as_str()],
            )
            .await
            .map(|_| ())
    }
}

const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS files (
    id             INTEGER PRIMARY KEY,
    path           TEXT    NOT NULL UNIQUE,
    filename       TEXT    NOT NULL,
    extension      TEXT,
    size_bytes     INTEGER,
    mtime          INTEGER NOT NULL,
    inode          INTEGER,
    content_hash   TEXT,
    file_kind      TEXT    NOT NULL,
    indexed_at     INTEGER NOT NULL,
    thumbnail_path TEXT
) STRICT;

CREATE INDEX IF NOT EXISTS files_path_idx ON files(path);
CREATE INDEX IF NOT EXISTS files_mtime_idx ON files(mtime);

CREATE TABLE IF NOT EXISTS metadata (
    id         INTEGER PRIMARY KEY,
    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    namespace  TEXT    NOT NULL,
    key        TEXT    NOT NULL,
    value      TEXT,
    data_type  TEXT    NOT NULL DEFAULT 'text',
    updated_at INTEGER NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS metadata_file_id_idx ON metadata(file_id);
CREATE INDEX IF NOT EXISTS metadata_namespace_key_idx ON metadata(namespace, key);
CREATE UNIQUE INDEX IF NOT EXISTS metadata_file_ns_key_idx ON metadata(file_id, namespace, key);
;

CREATE TABLE IF NOT EXISTS columns (
    id          INTEGER PRIMARY KEY,
    label       TEXT    NOT NULL,
    namespace   TEXT    NOT NULL,
    key         TEXT    NOT NULL,
    data_type   TEXT    NOT NULL,
    write_dest  TEXT    NOT NULL DEFAULT 'embedded_xmp',
    width_px    INTEGER NOT NULL DEFAULT 160,
    is_sortable INTEGER NOT NULL DEFAULT 1,
    is_editable INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS columns_ns_key_idx ON columns(namespace, key);

CREATE TABLE IF NOT EXISTS views (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL UNIQUE,
    columns_json TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS folder_views (
    path              TEXT    NOT NULL PRIMARY KEY,
    view_id           INTEGER NOT NULL REFERENCES views(id),
    apply_to_children INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE IF NOT EXISTS write_queue (
    id           INTEGER PRIMARY KEY,
    file_id      INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    namespace    TEXT    NOT NULL,
    key          TEXT    NOT NULL,
    old_value    TEXT,
    new_value    TEXT,
    status       TEXT    NOT NULL DEFAULT 'pending',
    error_msg    TEXT,
    created_at   INTEGER NOT NULL,
    completed_at INTEGER
) STRICT;

CREATE INDEX IF NOT EXISTS write_queue_status_idx ON write_queue(status);
CREATE INDEX IF NOT EXISTS write_queue_file_id_idx ON write_queue(file_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn schema_creates_expected_tables() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).await.unwrap();

        let mut rows = db
            .conn
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
                (),
            )
            .await
            .unwrap();

        let mut tables = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            let name = row.get_value(0).unwrap();
            tables.push(name.as_text().unwrap().to_string());
        }

        assert!(tables.contains(&"files".to_string()));
        assert!(tables.contains(&"metadata".to_string()));
        assert!(tables.contains(&"columns".to_string()));
        assert!(tables.contains(&"views".to_string()));
        assert!(tables.contains(&"write_queue".to_string()));
        assert!(tables.contains(&"folder_views".to_string()));
    }

    #[tokio::test]
    async fn mvcc_journal_mode_enabled() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).await.unwrap();

        let mut rows = db.conn.query("PRAGMA journal_mode", ()).await.unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let mode = row.get_value(0).unwrap();
        assert_eq!(mode.as_text().unwrap(), "mvcc");
    }

    #[tokio::test]
    async fn file_insert_query_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).await.unwrap();

        db.conn
            .execute(
                "INSERT INTO files (path, filename, extension, size_bytes, mtime, file_kind, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                ["/tmp/test.txt", "test.txt", "txt", "32", "1234", "document", "5678"],
            )
            .await
            .unwrap();

        let mut rows = db
            .conn
            .query(
                "SELECT filename FROM files WHERE path = ?1",
                ["/tmp/test.txt"],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let filename = row.get_value(0).unwrap();
        assert_eq!(filename.as_text().unwrap(), "test.txt");

        db.conn
            .execute("DELETE FROM files WHERE path = ?1", ["/tmp/test.txt"])
            .await
            .unwrap();

        let mut rows = db
            .conn
            .query(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                ["/tmp/test.txt"],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let count = row.get_value(0).unwrap();
        assert_eq!(count.as_integer().unwrap(), &0);
    }

}
