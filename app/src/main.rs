// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use valcore::{AccountView, Engine, FriendView, PlayerStats, Snapshot};

#[tauri::command]
async fn snapshot(engine: tauri::State<'_, Engine>) -> Result<Snapshot, String> {
    Ok(engine.snapshot().await)
}

#[tauri::command]
fn set_advanced(engine: tauri::State<'_, Engine>, on: bool) {
    engine.set_advanced(on);
}

#[tauri::command]
async fn player_stats(engine: tauri::State<'_, Engine>, puuid: String) -> Result<PlayerStats, String> {
    engine.player_stats(puuid).await
}

#[tauri::command]
async fn friends(engine: tauri::State<'_, Engine>) -> Result<Vec<FriendView>, String> {
    engine.friends().await
}

#[tauri::command]
async fn account(engine: tauri::State<'_, Engine>, force: bool) -> Result<AccountView, String> {
    engine.account(force).await
}

fn main() {
    tauri::Builder::default()
        .manage(Engine::new())
        .invoke_handler(tauri::generate_handler![snapshot, set_advanced, player_stats, friends, account])
        .run(tauri::generate_context!())
        .expect("uygulama başlatılamadı");
}
