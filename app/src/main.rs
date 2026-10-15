// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use valcore::updater::{self, UpdateInfo};
use valcore::{AccountView, CatalogSkin, Engine, FriendView, Inventory, PlayerStats, Scoreboard, Snapshot};

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
async fn match_detail(engine: tauri::State<'_, Engine>, id: String) -> Result<Scoreboard, String> {
    engine.match_detail(id).await
}

#[tauri::command]
async fn friends(engine: tauri::State<'_, Engine>) -> Result<Vec<FriendView>, String> {
    engine.friends().await
}

#[tauri::command]
async fn account(engine: tauri::State<'_, Engine>, force: bool) -> Result<AccountView, String> {
    engine.account(force).await
}

#[tauri::command]
async fn inventory(engine: tauri::State<'_, Engine>, force: bool) -> Result<Inventory, String> {
    engine.inventory(force).await
}

#[tauri::command]
async fn catalog(engine: tauri::State<'_, Engine>) -> Result<Vec<CatalogSkin>, String> {
    engine.catalog().await
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn check_update() -> Result<Option<UpdateInfo>, String> {
    updater::check(env!("CARGO_PKG_VERSION")).await
}

#[tauri::command]
async fn apply_update(app: tauri::AppHandle, info: UpdateInfo) -> Result<(), String> {
    let exe = updater::install(&info).await?;
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("Yeni sürüm başlatılamadı: {e}"))?;
    app.exit(0);
    Ok(())
}

fn main() {
    updater::cleanup();
    tauri::Builder::default()
        .manage(Engine::new())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            set_advanced,
            player_stats,
            match_detail,
            friends,
            account,
            inventory,
            catalog,
            app_version,
            check_update,
            apply_update
        ])
        .run(tauri::generate_context!())
        .expect("uygulama başlatılamadı");
}
