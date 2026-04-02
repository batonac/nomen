use crate::db::Database;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tokio::sync::Mutex;
use tauri::State;

#[derive(Debug, Serialize, Clone)]
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
    pub metadata: std::collections::HashMap<String, Option<String>>,
}

#[allow(dead_code)] // db will be used when commands query the index
pub struct AppState {
    pub db: Mutex<Database>,
}

fn classify_file_kind(path: &Path, is_dir: bool) -> &'static str {
    if is_dir {
        return "folder";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "svg" | "heic" | "heif" | "avif" | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2") => "image",
        Some("mp3" | "flac" | "wav" | "aac" | "ogg" | "opus" | "m4a" | "wma" | "aiff" | "alac") => "audio",
        Some("mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ts") => "video",
        Some("pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "txt" | "md" | "rtf" | "csv" | "json" | "xml" | "html" | "htm" | "tex" | "epub") => "document",
        _ => "other",
    }
}

#[tauri::command]
pub async fn navigate_to(
    path: String,
    _state: State<'_, AppState>,
) -> Result<Vec<FileRow>, String> {
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {}", e))?;

    let mut rows: Vec<FileRow> = Vec::new();
    let mut id_counter: i64 = 1;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();

        let extension = if is_dir {
            None
        } else {
            entry_path
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

        let file_kind = classify_file_kind(&entry_path, is_dir);

        rows.push(FileRow {
            id: id_counter,
            path: entry_path.to_string_lossy().to_string(),
            filename,
            extension,
            size_bytes,
            mtime,
            file_kind: file_kind.to_string(),
            thumbnail_path: None,
            metadata: std::collections::HashMap::new(),
        });

        id_counter += 1;
    }

    // Sort: folders first, then alphabetically by filename
    rows.sort_by(|a, b| {
        let a_is_folder = a.file_kind == "folder";
        let b_is_folder = b.file_kind == "folder";
        b_is_folder
            .cmp(&a_is_folder)
            .then_with(|| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()))
    });

    // Reassign IDs after sort
    for (i, row) in rows.iter_mut().enumerate() {
        row.id = (i + 1) as i64;
    }

    Ok(rows)
}
