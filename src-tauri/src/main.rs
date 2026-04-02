#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod exiftool;

use commands::AppState;
use db::Database;
use exiftool::daemon::ResilientExifToolDaemon;
use tokio::sync::Mutex;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let database = rt
        .block_on(Database::open_default())
        .expect("Failed to open database");

    let exiftool = ResilientExifToolDaemon::new("exiftool");

    let state = AppState {
        db: Mutex::new(database),
        exiftool,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
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
