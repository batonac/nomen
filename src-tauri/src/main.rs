#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use db::Database;
use tokio::sync::Mutex;
use tauri::Manager;

struct AppState {
    db: Mutex<Database>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Nomen is running.", name)
}

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
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
