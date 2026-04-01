export const SCHEMA_VERSION = 1;

const initialSchema = [
	"PRAGMA foreign_keys = ON;",
	`CREATE TABLE IF NOT EXISTS files (\n\
	  id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
	  path TEXT NOT NULL UNIQUE,\n\
	  filename TEXT NOT NULL,\n\
	  extension TEXT,\n\
	  size_bytes INTEGER,\n\
	  mtime INTEGER NOT NULL,\n\
	  inode INTEGER,\n\
	  content_hash TEXT,\n\
	  file_kind TEXT NOT NULL,\n\
	  indexed_at INTEGER NOT NULL,\n\
	  thumbnail_path TEXT\n\
	);`,
	"CREATE INDEX IF NOT EXISTS files_path_idx ON files(path);",
	"CREATE INDEX IF NOT EXISTS files_mtime_idx ON files(mtime);",
	`CREATE TABLE IF NOT EXISTS metadata (\n\
	  id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
	  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,\n\
	  namespace TEXT NOT NULL,\n\
	  key TEXT NOT NULL,\n\
	  value TEXT,\n\
	  data_type TEXT NOT NULL DEFAULT 'text',\n\
	  updated_at INTEGER NOT NULL\n\
	);`,
	"CREATE INDEX IF NOT EXISTS metadata_file_id_idx ON metadata(file_id);",
	"CREATE INDEX IF NOT EXISTS metadata_namespace_key_idx ON metadata(namespace, key);",
	"CREATE UNIQUE INDEX IF NOT EXISTS metadata_file_ns_key_idx ON metadata(file_id, namespace, key);",
	`CREATE TABLE IF NOT EXISTS columns (\n\
	  id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
	  label TEXT NOT NULL,\n\
	  namespace TEXT NOT NULL,\n\
	  key TEXT NOT NULL,\n\
	  data_type TEXT NOT NULL,\n\
	  write_dest TEXT NOT NULL DEFAULT 'embedded_xmp',\n\
	  width_px INTEGER NOT NULL DEFAULT 160,\n\
	  is_sortable INTEGER NOT NULL DEFAULT 1,\n\
	  is_editable INTEGER NOT NULL DEFAULT 1,\n\
	  created_at INTEGER NOT NULL\n\
	);`,
	"CREATE UNIQUE INDEX IF NOT EXISTS columns_ns_key_idx ON columns(namespace, key);",
	`CREATE TABLE IF NOT EXISTS views (\n\
	  id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
	  name TEXT NOT NULL UNIQUE,\n\
	  columns_json TEXT NOT NULL,\n\
	  created_at INTEGER NOT NULL,\n\
	  updated_at INTEGER NOT NULL\n\
	);`,
	`CREATE TABLE IF NOT EXISTS folder_views (\n\
	  path TEXT NOT NULL PRIMARY KEY,\n\
	  view_id INTEGER NOT NULL REFERENCES views(id),\n\
	  apply_to_children INTEGER NOT NULL DEFAULT 0\n\
	);`,
	`CREATE TABLE IF NOT EXISTS write_queue (\n\
	  id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
	  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,\n\
	  namespace TEXT NOT NULL,\n\
	  key TEXT NOT NULL,\n\
	  old_value TEXT,\n\
	  new_value TEXT,\n\
	  status TEXT NOT NULL DEFAULT 'pending',\n\
	  error_msg TEXT,\n\
	  created_at INTEGER NOT NULL,\n\
	  completed_at INTEGER\n\
	);`,
	"CREATE INDEX IF NOT EXISTS write_queue_status_idx ON write_queue(status);",
	"CREATE INDEX IF NOT EXISTS write_queue_file_id_idx ON write_queue(file_id);",
];

export const schemaMigrations: Record<number, string[]> = {
	1: initialSchema,
};