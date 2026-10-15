//! Names and pictures for agents, weapons and skins, from valorant-api.com.
//! Downloaded once per launch; nothing is sent to it except the request.

use serde_json::Value;
use std::collections::HashMap;

const LANG: &str = "tr-TR";

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Weapon {
    pub name: String,
    pub category: String,
    pub icon: Option<String>,
    pub default_skin: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub name: String,
    pub icon: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Chroma {
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Default)]
pub struct Content {
    pub agents: HashMap<String, Agent>,
    pub weapons: HashMap<String, Weapon>,
    pub skins: HashMap<String, Skin>,
    pub chromas: HashMap<String, Chroma>,
    /// content tier uuid -> highlight colour (#rrggbb)
    pub tiers: HashMap<String, String>,
    /// map path (/Game/Maps/...) -> display name
    pub maps: HashMap<String, String>,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
}

fn id(v: &Value, key: &str) -> Option<String> {
    s(v, key).map(|x| x.to_lowercase())
}

fn items(v: &Value) -> impl Iterator<Item = &Value> {
    v.get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// valorant-api colours are RRGGBBAA.
fn tier_colour(raw: &str) -> String {
    let hex: String = raw.chars().take(6).collect();
    format!("#{hex}")
}

impl Content {
    pub fn from_values(agents: &Value, weapons: &Value, tiers: &Value, maps: &Value) -> Self {
        let mut c = Content::default();

        for a in items(agents) {
            if let (Some(uuid), Some(name)) = (id(a, "uuid"), s(a, "displayName")) {
                let icon = s(a, "displayIconSmall").or_else(|| s(a, "displayIcon"));
                c.agents.insert(uuid, Agent { name, icon });
            }
        }

        for w in items(weapons) {
            let Some(wid) = id(w, "uuid") else { continue };
            let category = s(w, "category")
                .map(|x| x.rsplit("::").next().unwrap_or(x.as_str()).to_string())
                .unwrap_or_default();
            c.weapons.insert(
                wid,
                Weapon {
                    name: s(w, "displayName").unwrap_or_default(),
                    category,
                    icon: s(w, "displayIcon"),
                    default_skin: id(w, "defaultSkinUuid"),
                },
            );
            for sk in w.get("skins").and_then(Value::as_array).into_iter().flatten() {
                let Some(sid) = id(sk, "uuid") else { continue };
                let first_level_icon = sk
                    .pointer("/levels/0/displayIcon")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                c.skins.insert(
                    sid,
                    Skin {
                        name: s(sk, "displayName").unwrap_or_default(),
                        icon: s(sk, "displayIcon").or(first_level_icon),
                        tier: id(sk, "contentTierUuid"),
                    },
                );
                for ch in sk.get("chromas").and_then(Value::as_array).into_iter().flatten() {
                    if let Some(cid) = id(ch, "uuid") {
                        c.chromas.insert(
                            cid,
                            Chroma {
                                name: s(ch, "displayName").unwrap_or_default(),
                                icon: s(ch, "fullRender").or_else(|| s(ch, "displayIcon")),
                            },
                        );
                    }
                }
            }
        }

        for t in items(tiers) {
            if let (Some(uuid), Some(col)) = (id(t, "uuid"), s(t, "highlightColor")) {
                c.tiers.insert(uuid, tier_colour(&col));
            }
        }

        for m in items(maps) {
            if let (Some(url), Some(name)) = (s(m, "mapUrl"), s(m, "displayName")) {
                c.maps.insert(url, name);
            }
        }
        c
    }

    pub async fn download(http: &reqwest::Client) -> Result<Self, String> {
        async fn get(http: &reqwest::Client, url: String) -> Result<Value, String> {
            http.get(url)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())
        }
        let base = "https://valorant-api.com/v1";
        let agents = get(http, format!("{base}/agents?isPlayableCharacter=true&language={LANG}")).await?;
        let weapons = get(http, format!("{base}/weapons?language={LANG}")).await?;
        let tiers = get(http, format!("{base}/contenttiers?language={LANG}")).await?;
        let maps = get(http, format!("{base}/maps?language={LANG}")).await?;
        let c = Self::from_values(&agents, &weapons, &tiers, &maps);
        if c.weapons.is_empty() {
            return Err("Silah listesi boş geldi".into());
        }
        Ok(c)
    }
}
