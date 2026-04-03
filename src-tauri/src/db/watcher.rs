use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::mpsc;

use crate::db::Database;
use crate::exiftool::daemon::ResilientExifToolDaemon;

/// Watch `folder_path` for changes and update the index accordingly.
///
/// This function blocks until the returned `WatchHandle` is dropped (which
/// stops the watcher).  Call it from a `tokio::spawn` task.
#[allow(dead_code)]
pub struct WatchHandle {
    _watcher: RecommendedWatcher,
}

#[allow(dead_code)]
pub async fn watch_folder(
    folder_path: PathBuf,
    db: Arc<tokio::sync::Mutex<Database>>,
    daemon: Arc<ResilientExifToolDaemon>,
    app: AppHandle,
) -> Result<WatchHandle, String> {
    let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(128);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.blocking_send(res);
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )
    .map_err(|e| format!("Watcher init error: {e}"))?;

    watcher
        .watch(&folder_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Watcher watch error: {e}"))?;

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let event = match event {
                Ok(e) => e,
                Err(_) => continue,
            };
            handle_event(event, &db, &daemon, &app).await;
        }
    });

    Ok(WatchHandle { _watcher: watcher })
}

#[allow(dead_code)]
async fn handle_event(
    event: Event,
    db: &Arc<tokio::sync::Mutex<Database>>,
    daemon: &Arc<ResilientExifToolDaemon>,
    app: &AppHandle,
) {
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};

    match event.kind {
        EventKind::Create(CreateKind::File) | EventKind::Modify(ModifyKind::Data(_)) => {
            for path in &event.paths {
                let path_str = path.to_string_lossy().to_string();
                let db_guard = db.lock().await;
                re_index_file(&db_guard, daemon, &path_str).await;
                // Emit update so the grid can refresh.
                let _ = app.emit("index-update", serde_json::json!({ "path": path_str }));
            }
        }
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            for path in &event.paths {
                let path_str = path.to_string_lossy().to_string();
                let db_guard = db.lock().await;
                let _ = db_guard.delete_file_by_path(&path_str).await;
                let _ = app.emit("index-update", serde_json::json!({ "path": path_str, "deleted": true }));
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            // event.paths[0] = old, event.paths[1] = new
            if event.paths.len() >= 2 {
                let old = event.paths[0].to_string_lossy().to_string();
                let new = event.paths[1].to_string_lossy().to_string();
                let new_filename = event.paths[1]
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let db_guard = db.lock().await;
                let _ = db_guard.rename_file(&old, &new, &new_filename).await;
                let _ = app.emit("index-update", serde_json::json!({ "oldPath": old, "newPath": new }));
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
async fn re_index_file(
    db: &Database,
    daemon: &ResilientExifToolDaemon,
    path_str: &str,
) {
    use std::fs;
    use std::path::Path;
    use std::time::UNIX_EPOCH;

    let path = Path::new(path_str);
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir = meta.is_dir();
    let extension: Option<String> = if is_dir {
        None
    } else {
        path.extension()
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

    #[cfg(target_family = "unix")]
    let inode: Option<i64> = {
        use std::os::unix::fs::MetadataExt;
        Some(meta.ino() as i64)
    };
    #[cfg(not(target_family = "unix"))]
    let inode: Option<i64> = None;

    let file_kind = crate::db::indexer::classify_file_kind_pub(path, is_dir);
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let file_id = match db
        .upsert_file(path_str, &filename, extension.as_deref(), size_bytes, mtime, inode, file_kind, now_ms)
        .await
    {
        Ok(id) => id,
        Err(_) => return,
    };

    if !is_dir {
        if let Ok(exif) = crate::exiftool::extract::extract_metadata(daemon, path_str).await {
            for (key, value) in &exif.tags {
                let (ns, tag_key) = if let Some(pos) = key.find(':') {
                    (&key[..pos], &key[pos + 1..])
                } else {
                    ("system", key.as_str())
                };
                let _ = db.upsert_metadata(file_id, ns, tag_key, value.as_deref(), now_ms).await;
            }
        }
    }
}
