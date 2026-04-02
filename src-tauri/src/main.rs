#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod exiftool;

use commands::AppState;
use db::Database;
use exiftool::daemon::ResilientExifToolDaemon;
use std::sync::Arc;
use tokio::sync::Mutex;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let database = rt
        .block_on(Database::open_default())
        .expect("Failed to open database");

    let db_arc: Arc<Mutex<Database>> = Arc::new(Mutex::new(database));
    let exiftool_arc: Arc<ResilientExifToolDaemon> =
        Arc::new(ResilientExifToolDaemon::new("exiftool"));

    let state = AppState {
        db: Arc::clone(&db_arc),
        exiftool: Arc::clone(&exiftool_arc),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .setup(move |app| {
            // Start the write-back queue worker as a background task.
            let db_clone = Arc::clone(&db_arc);
            let exiftool_clone = Arc::clone(&exiftool_arc);
            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                db::write_worker::run_write_worker(db_clone, exiftool_clone, app_handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::navigate_to,
            commands::get_metadata,
            commands::write_metadata,
            commands::bulk_write,
            commands::file_op,
            commands::add_column,
            commands::get_views,
            commands::save_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
