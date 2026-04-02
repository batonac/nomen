use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use tauri::{AppHandle, Emitter};

use crate::db::Database;
use crate::exiftool::extract::extract_metadata;
use crate::exiftool::daemon::ResilientExifToolDaemon;

/// Progress event payload emitted during a scan.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub folder_path: String,
    pub total: usize,
    pub indexed: usize,
    pub phase: String,
}

/// Index the direct children of `folder_path`:
/// 1. Walk directory entries, upsert filesystem metadata into `files` table.
/// 2. For each new or changed file, run ExifTool extraction and upsert `metadata`.
/// 3. Emit `index-progress` events via the Tauri app handle.
///
/// Only direct children are indexed (not recursive).
pub async fn index_folder(
    db: &Database,
    daemon: &ResilientExifToolDaemon,
    app: &AppHandle,
    folder_path: &str,
) -> Result<(), String> {
    let dir = Path::new(folder_path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {folder_path}"));
    }

    let entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .filter_map(|e| e.ok())
        .collect();

    let total = entries.len();

    let _ = app.emit(
        "index-progress",
        IndexProgress {
            folder_path: folder_path.to_string(),
            total,
            indexed: 0,
            phase: "scanning".to_string(),
        },
    );

    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for (idx, entry) in entries.iter().enumerate() {
        let path_str = entry.path().to_string_lossy().to_string();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let filename = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let extension: Option<String> = if is_dir {
            None
        } else {
            entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
        };
        let size_bytes: Option<i64> = if is_dir { None } else { Some(meta.len() as i64) };
        let mtime = meta
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let inode = inode_from_meta(&meta);
        let file_kind = classify_file_kind_pub(&entry.path(), is_dir);

        // Check whether this file is already in the index and unchanged.
        let existing = db
            .get_file_by_path(&path_str)
            .await
            .unwrap_or(None);

        let file_id = db
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
            .map_err(|e| format!("DB upsert error: {e}"))?;

        // Only run ExifTool for non-directory files that have changed.
        let needs_extraction = !is_dir && match &existing {
            None => true,
            Some(row) => row.mtime != mtime || row.inode != inode,
        };

        if needs_extraction {
            match extract_metadata(daemon, &path_str).await {
                Ok(exif) => {
                    for (key, value) in &exif.tags {
                        // Split "Namespace:Key" into parts.
                        let (ns, tag_key) = split_namespace_key(key);
                        let _ = db
                            .upsert_metadata(file_id, ns, tag_key, value.as_deref(), now_ms)
                            .await;
                    }
                }
                Err(_) => {
                    // ExifTool failed for this file — not fatal; skip metadata.
                }
            }
        }

        let _ = app.emit(
            "index-progress",
            IndexProgress {
                folder_path: folder_path.to_string(),
                total,
                indexed: idx + 1,
                phase: "extracting".to_string(),
            },
        );
    }

    let _ = app.emit(
        "index-progress",
        IndexProgress {
            folder_path: folder_path.to_string(),
            total,
            indexed: total,
            phase: "complete".to_string(),
        },
    );

    Ok(())
}

fn split_namespace_key(key: &str) -> (&str, &str) {
    if let Some(pos) = key.find(':') {
        (&key[..pos], &key[pos + 1..])
    } else {
        ("system", key)
    }
}

/// Public wrapper for use by the watcher module.
pub fn classify_file_kind_pub(path: &Path, is_dir: bool) -> &'static str {
    if is_dir {
        return "folder";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some(
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "svg" | "heic"
            | "heif" | "avif" | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2",
        ) => "image",
        Some(
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "opus" | "m4a" | "wma" | "aiff" | "alac",
        ) => "audio",
        Some("mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ts") => "video",
        Some(
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
            | "txt" | "md" | "rtf" | "csv" | "json" | "xml" | "html" | "htm" | "tex" | "epub",
        ) => "document",
        _ => "other",
    }
}

#[cfg(target_family = "unix")]
fn inode_from_meta(meta: &fs::Metadata) -> Option<i64> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.ino() as i64)
}

#[cfg(not(target_family = "unix"))]
fn inode_from_meta(_meta: &fs::Metadata) -> Option<i64> {
    None
}
