use std::collections::HashMap;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::db::Database;
use crate::exiftool::daemon::ResilientExifToolDaemon;
use crate::exiftool::write::{write_metadata, MetadataWrite};

/// Poll the `write_queue` table every 500 ms and flush pending writes.
///
/// Runs forever; call from a `tokio::spawn` task.
pub async fn run_write_worker(
    db: Arc<Mutex<Database>>,
    daemon: Arc<ResilientExifToolDaemon>,
    app: AppHandle,
) {
    loop {
        sleep(Duration::from_millis(500)).await;
        process_pending(&db, &daemon, &app).await;
    }
}

/// Collect all `pending` rows, group by file, issue one ExifTool call per
/// file, then mark rows `complete` or `failed`.
async fn process_pending(
    db: &Arc<Mutex<Database>>,
    daemon: &ResilientExifToolDaemon,
    app: &AppHandle,
) {
    let rows = {
        let guard = db.lock().await;
        match guard.get_pending_writes().await {
            Ok(r) => r,
            Err(_) => return,
        }
    };

    if rows.is_empty() {
        return;
    }

    // Group by file path.
    let mut by_path: HashMap<String, Vec<PendingRow>> = HashMap::new();
    for row in rows {
        by_path.entry(row.file_path.clone()).or_default().push(row);
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for (path, rows) in by_path {
        let writes: Vec<MetadataWrite> = rows
            .iter()
            .map(|r| MetadataWrite {
                tag: format!("{}:{}", r.namespace, r.key),
                value: r.new_value.clone(),
            })
            .collect();

        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

        let result = write_metadata(daemon, &path, &writes).await;

        let guard = db.lock().await;
        match result {
            Ok(wr) if wr.success => {
                for id in &ids {
                    let _ = guard.mark_write_complete(*id, now_ms).await;
                }

                // Refresh metadata in index from ExifTool.
                if let Ok(exif) =
                    crate::exiftool::extract::extract_metadata(daemon, &path).await
                {
                    let file_id = rows[0].file_id;
                    for (key, value) in &exif.tags {
                        let (ns, tag_key) = if let Some(pos) = key.find(':') {
                            (&key[..pos], &key[pos + 1..])
                        } else {
                            ("system", key.as_str())
                        };
                        let _ = guard
                            .upsert_metadata(file_id, ns, tag_key, value.as_deref(), now_ms)
                            .await;
                    }
                }

                let _ = app.emit(
                    "write-result",
                    serde_json::json!({
                        "success": true,
                        "path": path,
                        "affectedFiles": 1,
                        "failedFiles": 0,
                        "errors": []
                    }),
                );
            }
            Ok(wr) => {
                for id in &ids {
                    let _ = guard.mark_write_failed(*id, &wr.message, now_ms).await;
                }
                let _ = app.emit(
                    "write-result",
                    serde_json::json!({
                        "success": false,
                        "path": path,
                        "affectedFiles": 0,
                        "failedFiles": 1,
                        "errors": [{ "path": path, "message": wr.message }]
                    }),
                );
            }
            Err(e) => {
                for id in &ids {
                    let _ = guard.mark_write_failed(*id, &e, now_ms).await;
                }
                let _ = app.emit(
                    "write-result",
                    serde_json::json!({
                        "success": false,
                        "path": path,
                        "affectedFiles": 0,
                        "failedFiles": 1,
                        "errors": [{ "path": path, "message": e }]
                    }),
                );
            }
        }
    }
}

/// A pending write row as returned by the database.
pub struct PendingRow {
    pub id: i64,
    pub file_id: i64,
    pub file_path: String,
    pub namespace: String,
    pub key: String,
    pub new_value: Option<String>,
}
