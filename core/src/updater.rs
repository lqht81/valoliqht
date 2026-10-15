//! Checks GitHub for a newer release and swaps the running exe for it.
//!
//! The only request is to the repository's public "latest release" page and,
//! when you press update, the download of the exe attached to it. The file's
//! SHA-256 is checked against the one GitHub publishes for the asset.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

const REPO: &str = "lqht81/valoliqht";
const ASSET: &str = "ValoLiqht-portable.exe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub notes: Option<String>,
    /// "sha256:<hex>" as published by GitHub.
    pub digest: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("ValoLiqht")
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((parts.next()??, parts.next().flatten().unwrap_or(0), parts.next().flatten().unwrap_or(0)))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Reads the latest-release JSON from GitHub's API.
pub fn from_release(v: &Value, current: &str) -> Option<UpdateInfo> {
    let tag = v.get("tag_name")?.as_str()?;
    if !is_newer(tag, current) {
        return None;
    }
    let asset = v
        .get("assets")?
        .as_array()?
        .iter()
        .find(|a| a.get("name").and_then(Value::as_str) == Some(ASSET))?;
    Some(UpdateInfo {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        url: asset.get("browser_download_url")?.as_str()?.to_string(),
        notes: v.get("body").and_then(Value::as_str).map(str::to_string),
        digest: asset.get("digest").and_then(Value::as_str).map(str::to_string),
    })
}

pub async fn check(current: &str) -> Result<Option<UpdateInfo>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Güncelleme kontrol edilemedi: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let v: Value = resp.error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    Ok(from_release(&v, current))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn verify(bytes: &[u8], digest: Option<&str>) -> Result<(), String> {
    let Some(d) = digest else { return Ok(()) };
    let Some(expected) = d.strip_prefix("sha256:") else { return Ok(()) };
    let actual = hex(&Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("İndirilen dosya doğrulanamadı, güncelleme iptal edildi.".into())
    }
}

fn old_path(exe: &std::path::Path) -> PathBuf {
    exe.with_extension("old")
}

/// Downloads the new exe and puts it in place of the running one.
/// Returns the path to start.
pub async fn install(info: &UpdateInfo) -> Result<PathBuf, String> {
    if !info.url.starts_with("https://github.com/") {
        return Err("Beklenmeyen indirme adresi.".into());
    }
    let bytes = client()?
        .get(&info.url)
        .send()
        .await
        .map_err(|e| format!("İndirilemedi: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() < 1_000_000 {
        return Err("İndirilen dosya eksik görünüyor.".into());
    }
    verify(&bytes, info.digest.as_deref())?;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let fresh = exe.with_extension("new");
    let old = old_path(&exe);
    std::fs::write(&fresh, &bytes).map_err(|e| format!("Yazılamadı: {e}"))?;
    let _ = std::fs::remove_file(&old);
    // A running exe can be renamed on Windows, just not deleted.
    std::fs::rename(&exe, &old).map_err(|e| format!("Eski sürüm taşınamadı: {e}"))?;
    if let Err(e) = std::fs::rename(&fresh, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("Yeni sürüm yerleştirilemedi: {e}"));
    }
    Ok(exe)
}

/// Removes the previous exe left behind by an update.
pub fn cleanup() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_path(&exe));
        let _ = std::fs::remove_file(exe.with_extension("new"));
    }
}
