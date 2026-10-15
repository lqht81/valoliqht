// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use valcore::{Engine, Snapshot};

#[tauri::command]
async fn snapshot(engine: tauri::State<'_, Engine>) -> Result<Snapshot, String> {
    Ok(engine.snapshot().await)
}

fn main() {
    tauri::Builder::default()
        .manage(Engine::new())
        .invoke_handler(tauri::generate_handler![snapshot])
        .run(tauri::generate_context!())
        .expect("uygulama başlatılamadı");
}
