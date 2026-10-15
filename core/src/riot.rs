//! Talks to the Riot Client on this PC and to Riot's own game servers.
//! Nothing here contacts any other host.

use crate::local::{shard_for_region, Lockfile, LogInfo};
use serde_json::Value;
use std::time::Duration;

/// Base64 of the platform JSON every VALORANT client sends.
const CLIENT_PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

pub type Res<T> = Result<T, String>;

#[derive(Debug, Clone)]
pub struct Session {
    pub puuid: String,
    access_token: String,
    entitlement: String,
    pub region: String,
    pub shard: String,
    version: String,
}

pub struct Riot {
    /// Only ever used for https://127.0.0.1 — the Riot Client signs that
    /// endpoint with its own certificate.
    local: reqwest::Client,
    remote: reqwest::Client,
}

impl Default for Riot {
    fn default() -> Self {
        Self::new()
    }
}

impl Riot {
    pub fn new() -> Self {
        let local = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .timeout(Duration::from_secs(4))
            .build()
            .expect("local http client");
        let remote = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .expect("remote http client");
        Self { local, remote }
    }

    async fn local_get(&self, lf: &Lockfile, path: &str) -> Res<Value> {
        let url = format!("https://127.0.0.1:{}{}", lf.port, path);
        let resp = self
            .local
            .get(url)
            .basic_auth("riot", Some(&lf.password))
            .send()
            .await
            .map_err(|e| format!("Riot Client'a bağlanılamadı: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Riot Client {} döndü", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn session(&self, lf: &Lockfile, log: &LogInfo) -> Res<Session> {
        let ent = self.local_get(lf, "/entitlements/v1/token").await?;
        let puuid = str_at(&ent, "subject").ok_or("Riot hesabına giriş yapılmamış")?;
        let access_token = str_at(&ent, "accessToken").ok_or("Erişim anahtarı yok")?;
        let entitlement = str_at(&ent, "token").ok_or("Yetki anahtarı yok")?;

        let (region, shard) = match (&log.region, &log.shard) {
            (Some(r), Some(s)) => (r.clone(), s.clone()),
            _ => {
                let region = self
                    .region_from_launch_args(lf)
                    .await
                    .ok_or("VALORANT açık değil")?;
                let shard = shard_for_region(&region).to_string();
                (region, shard)
            }
        };

        let version = match &log.version {
            Some(v) => v.clone(),
            None => self.client_version().await?,
        };

        Ok(Session {
            puuid,
            access_token,
            entitlement,
            region,
            shard,
            version,
        })
    }

    async fn region_from_launch_args(&self, lf: &Lockfile) -> Option<String> {
        let sessions = self
            .local_get(lf, "/product-session/v1/external-sessions")
            .await
            .ok()?;
        for s in sessions.as_object()?.values() {
            if s.get("productId").and_then(Value::as_str) != Some("valorant") {
                continue;
            }
            let args = s.pointer("/launchConfiguration/arguments")?.as_array()?;
            for a in args.iter().filter_map(Value::as_str) {
                if let Some(r) = a.strip_prefix("-ares-deployment=") {
                    return Some(r.to_string());
                }
            }
        }
        None
    }

    async fn client_version(&self) -> Res<String> {
        let v: Value = self
            .remote
            .get("https://valorant-api.com/v1/version")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        v.pointer("/data/riotClientVersion")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "Oyun sürümü bulunamadı".to_string())
    }

    fn signed(&self, req: reqwest::RequestBuilder, s: &Session) -> reqwest::RequestBuilder {
        req.bearer_auth(&s.access_token)
            .header("X-Riot-Entitlements-JWT", &s.entitlement)
            .header("X-Riot-ClientPlatform", CLIENT_PLATFORM)
            .header("X-Riot-ClientVersion", &s.version)
    }

    /// GET on the game server. `Ok(None)` means 404 (e.g. not in that phase).
    pub async fn glz(&self, s: &Session, path: &str) -> Res<Option<Value>> {
        let url = format!("https://glz-{}-1.{}.a.pvp.net{}", s.region, s.shard, path);
        let resp = self
            .signed(self.remote.get(url), s)
            .send()
            .await
            .map_err(|e| format!("Riot sunucusuna ulaşılamadı: {e}"))?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(format!("Riot sunucusu {status} döndü"));
        }
        resp.json().await.map(Some).map_err(|e| e.to_string())
    }

    /// Riot IDs for a set of players.
    pub async fn names(&self, s: &Session, puuids: &[String]) -> Res<Vec<(String, String, String)>> {
        if puuids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("https://pd.{}.a.pvp.net/name-service/v2/players", s.shard);
        let resp = self
            .signed(self.remote.put(url), s)
            .json(puuids)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("İsim servisi {} döndü", resp.status()));
        }
        let list: Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(list
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        Some((
                            str_at(p, "Subject")?,
                            str_at(p, "GameName")?,
                            str_at(p, "TagLine").unwrap_or_default(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn remote(&self) -> &reqwest::Client {
        &self.remote
    }
}

pub fn str_at(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
