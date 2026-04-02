use turso::{Builder, Connection, Database as TursoDb};
use std::fs;
use std::path::PathBuf;

const SCHEMA_VERSION: i64 = 1;

pub struct Database {
    pub conn: Connection,
    pub path: PathBuf,
}

impl Database {
    pub async fn open_default() -> turso::Result<Self> {
        let base = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".nomen");
        fs::create_dir_all(&base).expect("Cannot create ~/.nomen directory");
        let path = base.join("index.db");
        Self::open(&path).await
    }

    pub async fn open(path: &PathBuf) -> turso::Result<Self> {
        let db: TursoDb = Builder::new_local(path.to_str().expect("Invalid path"))
            .build()
            .await?;
        let conn = db.connect()?;
        conn.execute("PRAGMA foreign_keys = ON", ()).await?;

        let database = Database {
            conn,
            path: path.clone(),
        };
        database.migrate().await?;
        Ok(database)
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
            // Execute each statement individually — Turso doesn't have execute_batch
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
}

const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS files (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    path          TEXT    NOT NULL UNIQUE,
    filename      TEXT    NOT NULL,
    extension     TEXT,
    size_bytes    INTEGER,
    mtime         INTEGER NOT NULL,
    inode         INTEGER,
    content_hash  TEXT,
    file_kind     TEXT    NOT NULL,
    indexed_at    INTEGER NOT NULL,
    thumbnail_path TEXT
);

CREATE INDEX IF NOT EXISTS files_path_idx ON files(path);
CREATE INDEX IF NOT EXISTS files_mtime_idx ON files(mtime);

CREATE TABLE IF NOT EXISTS metadata (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    namespace  TEXT    NOT NULL,
    key        TEXT    NOT NULL,
    value      TEXT,
    data_type  TEXT    NOT NULL DEFAULT 'text',
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS metadata_file_id_idx ON metadata(file_id);
CREATE INDEX IF NOT EXISTS metadata_namespace_key_idx ON metadata(namespace, key);
CREATE UNIQUE INDEX IF NOT EXISTS metadata_file_ns_key_idx ON metadata(file_id, namespace, key);

CREATE TABLE IF NOT EXISTS columns (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT    NOT NULL,
    namespace   TEXT    NOT NULL,
    key         TEXT    NOT NULL,
    data_type   TEXT    NOT NULL,
    write_dest  TEXT    NOT NULL DEFAULT 'embedded_xmp',
    width_px    INTEGER NOT NULL DEFAULT 160,
    is_sortable INTEGER NOT NULL DEFAULT 1,
    is_editable INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS columns_ns_key_idx ON columns(namespace, key);

CREATE TABLE IF NOT EXISTS views (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    columns_json TEXT   NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS folder_views (
    path              TEXT    NOT NULL PRIMARY KEY,
    view_id           INTEGER NOT NULL REFERENCES views(id),
    apply_to_children INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS write_queue (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id      INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    namespace    TEXT    NOT NULL,
    key          TEXT    NOT NULL,
    old_value    TEXT,
    new_value    TEXT,
    status       TEXT    NOT NULL DEFAULT 'pending',
    error_msg    TEXT,
    created_at   INTEGER NOT NULL,
    completed_at INTEGER
);

CREATE INDEX IF NOT EXISTS write_queue_status_idx ON write_queue(status);
CREATE INDEX IF NOT EXISTS write_queue_file_id_idx ON write_queue(file_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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
