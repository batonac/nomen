#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;

use commands::AppState;
use db::Database;
use tokio::sync::Mutex;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let database = rt
        .block_on(Database::open_default())
        .expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: Mutex::new(database),
        })
        .invoke_handler(tauri::generate_handler![commands::navigate_to])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
