use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tauri::{Emitter, State};
use tokio::sync::Mutex;

use crate::db::{ColumnRow, Database, FileRow, MetadataRow, ReadPool, ViewRow};
use crate::exiftool::daemon::ResilientExifToolDaemon;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub reads: Arc<ReadPool>,
    pub exiftool: Arc<ResilientExifToolDaemon>,
}

// ─── navigate_to ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn navigate_to(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<FileRow>, String> {
    use std::path::Path;

    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    {
        let db = state.db.lock().await;
        fast_index_folder(&db, &path).await?;
    }

    // Spawn full ExifTool scan in the background using Arc clones — no raw pointers.
    let db_clone = Arc::clone(&state.db);
    let exiftool_clone = Arc::clone(&state.exiftool);
    let app_clone = app.clone();
    let path_clone = path.clone();
    tauri::async_runtime::spawn(async move {
        let db = db_clone.lock().await;
        let _ = crate::db::indexer::index_folder(
            &db,
            &exiftool_clone,
            &app_clone,
            &path_clone,
        )
        .await;
        let _ = app_clone.emit("index-update", serde_json::json!({ "folderPath": path_clone }));
    });

    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_files_for_folder(&path)
        .await
        .map_err(|e| format!("DB error: {e}"))
}

/// Fast filesystem-only index (no ExifTool): upserts system metadata for all
/// direct children of `folder_path`.
async fn fast_index_folder(db: &Database, folder_path: &str) -> Result<(), String> {
    use std::fs;
    use std::path::Path;

    let dir = Path::new(folder_path);
    let entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .filter_map(|e| e.ok())
        .collect();

    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for entry in entries {
        let path_str = entry.path().to_string_lossy().to_string();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let filename = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let extension = if is_dir {
            None
        } else {
            entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
        };
        let size_bytes = if is_dir { None } else { Some(meta.len() as i64) };
        let mtime = meta
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        #[cfg(target_family = "unix")]
        let inode: Option<i64> = {
            use std::os::unix::fs::MetadataExt;
            Some(meta.ino() as i64)
        };
        #[cfg(not(target_family = "unix"))]
        let inode: Option<i64> = None;

        let file_kind = crate::db::indexer::classify_file_kind_pub(&entry.path(), is_dir);

        if let Err(e) = db
            .upsert_file(
                &path_str,
                &filename,
                extension.as_deref(),
                size_bytes,
                mtime,
                inode,
                file_kind,
                now_ms,
            )
            .await
        {
            eprintln!("[fast_index] upsert error: {e}");
        }
    }
    Ok(())
}

// ─── get_metadata ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_metadata(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<MetadataRow>, String> {
    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_metadata_for_file(file_id)
        .await
        .map_err(|e| format!("DB error: {e}"))
}

// ─── write_metadata ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataWriteInput {
    pub file_id: i64,
    pub namespace: String,
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResultOutput {
    pub success: bool,
    pub affected_files: usize,
    pub failed_files: usize,
    pub errors: Vec<WriteErrorOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteErrorOutput {
    pub file_id: i64,
    pub path: String,
    pub message: String,
}

#[tauri::command]
pub async fn write_metadata(
    writes: Vec<MetadataWriteInput>,
    state: State<'_, AppState>,
) -> Result<WriteResultOutput, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let db = state.db.lock().await;
    let mut errors = Vec::new();

    for w in &writes {
        let _ = db
            .upsert_metadata(w.file_id, &w.namespace, &w.key, w.new_value.as_deref(), now_ms)
            .await;

        if let Err(e) = db
            .enqueue_write(
                w.file_id,
                &w.namespace,
                &w.key,
                w.old_value.as_deref(),
                w.new_value.as_deref(),
                now_ms,
            )
            .await
        {
            errors.push(WriteErrorOutput {
                file_id: w.file_id,
                path: String::new(),
                message: e.to_string(),
            });
        }
    }

    Ok(WriteResultOutput {
        success: errors.is_empty(),
        affected_files: writes.len() - errors.len(),
        failed_files: errors.len(),
        errors,
    })
}

// ─── bulk_write ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkWriteInput {
    pub file_ids: Vec<i64>,
    pub namespace: String,
    pub key: String,
    pub value: Option<String>,
}

#[tauri::command]
pub async fn bulk_write(
    write: BulkWriteInput,
    state: State<'_, AppState>,
) -> Result<WriteResultOutput, String> {
    let individual: Vec<MetadataWriteInput> = write
        .file_ids
        .into_iter()
        .map(|id| MetadataWriteInput {
            file_id: id,
            namespace: write.namespace.clone(),
            key: write.key.clone(),
            old_value: None,
            new_value: write.value.clone(),
        })
        .collect();
    write_metadata(individual, state).await
}

// ─── file_op ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FileOperationInput {
    Open { paths: Vec<String> },
    Reveal { paths: Vec<String> },
    Rename { path: String, next_name: String },
    Move { paths: Vec<String>, destination: String },
    Copy { paths: Vec<String>, destination: String },
    Delete { paths: Vec<String> },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOpResultOutput {
    pub success: bool,
    pub message: Option<String>,
    pub paths: Option<Vec<String>>,
}

#[tauri::command]
pub async fn file_op(
    operation: FileOperationInput,
    state: State<'_, AppState>,
) -> Result<FileOpResultOutput, String> {
    match operation {
        FileOperationInput::Open { paths } => open_files(&paths),
        FileOperationInput::Reveal { paths } => reveal_files(&paths),
        FileOperationInput::Rename { path, next_name } => {
            let db = state.db.lock().await;
            rename_file_op(&db, &path, &next_name).await
        }
        FileOperationInput::Delete { paths } => {
            let db = state.db.lock().await;
            delete_files_op(&db, &paths).await
        }
        FileOperationInput::Move { paths, destination } => {
            let db = state.db.lock().await;
            move_files_op(&db, &paths, &destination, false).await
        }
        FileOperationInput::Copy { paths, destination } => {
            let db = state.db.lock().await;
            move_files_op(&db, &paths, &destination, true).await
        }
    }
}

fn open_files(paths: &[String]) -> Result<FileOpResultOutput, String> {
    for path in paths {
        #[cfg(target_os = "macos")]
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "windows")]
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(FileOpResultOutput { success: true, message: None, paths: None })
}

fn reveal_files(paths: &[String]) -> Result<FileOpResultOutput, String> {
    for path in paths {
        #[cfg(target_os = "macos")]
        std::process::Command::new("open")
            .args(["-R", path])
            .spawn()
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "linux")]
        {
            if std::process::Command::new("nautilus")
                .args(["--select", path])
                .spawn()
                .is_err()
            {
                let parent = std::path::Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                std::process::Command::new("xdg-open")
                    .arg(&parent)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
        }

        #[cfg(target_os = "windows")]
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(FileOpResultOutput { success: true, message: None, paths: None })
}

async fn rename_file_op(db: &Database, path: &str, next_name: &str) -> Result<FileOpResultOutput, String> {
    let old = std::path::Path::new(path);
    let new = old.with_file_name(next_name);
    std::fs::rename(old, &new).map_err(|e| e.to_string())?;
    let new_str = new.to_string_lossy().to_string();
    db.rename_file(path, &new_str, next_name)
        .await
        .map_err(|e| format!("DB rename error: {e}"))?;
    Ok(FileOpResultOutput {
        success: true,
        message: None,
        paths: Some(vec![new_str]),
    })
}

async fn delete_files_op(db: &Database, paths: &[String]) -> Result<FileOpResultOutput, String> {
    for path in paths {
        let p = std::path::Path::new(path);
        if p.is_dir() {
            std::fs::remove_dir_all(p).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(p).map_err(|e| e.to_string())?;
        }
        db.delete_file_by_path(path)
            .await
            .map_err(|e| format!("DB delete error: {e}"))?;
    }
    Ok(FileOpResultOutput { success: true, message: None, paths: None })
}

async fn move_files_op(
    db: &Database,
    paths: &[String],
    destination: &str,
    copy: bool,
) -> Result<FileOpResultOutput, String> {
    let dest_dir = std::path::Path::new(destination);
    let mut new_paths = Vec::new();

    for path in paths {
        let src = std::path::Path::new(path);
        let filename = src
            .file_name()
            .ok_or_else(|| format!("Cannot get filename from {path}"))?;
        let dest = dest_dir.join(filename);
        let dest_str = dest.to_string_lossy().to_string();

        if copy {
            std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
        } else {
            if std::fs::rename(src, &dest).is_err() {
                std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
                std::fs::remove_file(src).map_err(|e| e.to_string())?;
            }
            db.delete_file_by_path(path)
                .await
                .map_err(|e| format!("DB error: {e}"))?;
        }
        new_paths.push(dest_str);
    }

    Ok(FileOpResultOutput {
        success: true,
        message: None,
        paths: Some(new_paths),
    })
}

// ─── Column / view commands ───────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInput {
    pub label: String,
    pub namespace: String,
    pub key: String,
    pub data_type: String,
    pub write_dest: String,
    pub width_px: Option<i64>,
}

#[tauri::command]
pub async fn add_column(
    column: ColumnInput,
    state: State<'_, AppState>,
) -> Result<ColumnRow, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    {
        let db = state.db.lock().await;
        db.add_column(
            &column.label,
            &column.namespace,
            &column.key,
            &column.data_type,
            &column.write_dest,
            column.width_px.unwrap_or(160),
            now_ms,
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    }
    // Read the freshly-inserted row back so the frontend gets the assigned id.
    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_columns()
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .into_iter()
        .find(|c| c.namespace == column.namespace && c.key == column.key)
        .ok_or_else(|| "Column not found after insert".to_string())
}

#[tauri::command]
pub async fn add_columns(
    columns: Vec<ColumnInput>,
    state: State<'_, AppState>,
) -> Result<Vec<ColumnRow>, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    {
        let db = state.db.lock().await;
        for col in &columns {
            db.add_column(
                &col.label,
                &col.namespace,
                &col.key,
                &col.data_type,
                &col.write_dest,
                col.width_px.unwrap_or(160),
                now_ms,
            )
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        }
    }
    // Return the full updated column list so the frontend can replace its state.
    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_columns()
        .await
        .map_err(|e| format!("DB error: {e}"))
}

#[tauri::command]
pub async fn get_columns(state: State<'_, AppState>) -> Result<Vec<ColumnRow>, String> {
    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_columns()
        .await
        .map_err(|e| format!("DB error: {e}"))
}

#[tauri::command]
pub async fn get_views(state: State<'_, AppState>) -> Result<Vec<ViewRow>, String> {
    state.reads
        .reader()
        .map_err(|e| format!("DB error: {e}"))?
        .get_views()
        .await
        .map_err(|e| format!("DB error: {e}"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveViewInput {
    pub name: String,
    pub columns_json: String,
}

#[tauri::command]
pub async fn save_view(
    view: SaveViewInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let db = state.db.lock().await;
    db.save_view(&view.name, &view.columns_json, now_ms)
        .await
        .map_err(|e| format!("DB error: {e}"))
}
