// Prevents additional console window on Windows in release mode.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

/// Returns static application metadata.
/// This is the ONLY Tauri command in GUI-M0.
/// It does NOT access hardware, filesystem, network, or executor.
#[tauri::command]
fn app_metadata() -> serde_json::Value {
    serde_json::json!({
        "name": "ODMR GUI-M0 Mock Viewer",
        "version": "0.1.0",
        "phase": "M1 mock complete / M2 hardware bring-up pending",
        "mode": "MOCK ONLY",
        "boundary_note": "No hardware access. No executor connection. Real controls disabled."
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![app_metadata])
        .setup(|app| {
            // GUI-M0: no hardware initialization, no device discovery,
            // no executor connection, no file watchers.
            let _window = app.get_webview_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
