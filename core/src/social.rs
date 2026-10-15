//! Friends and presence from the local Riot Client.

use crate::content::{Content, TierBadge};
use crate::riot::str_at;
use crate::stats::queue_label;
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Presence {
    pub puuid: String,
    pub product: String,
    pub state: String,
    /// Decoded VALORANT presence JSON (Null for other games).
    pub data: Value,
}

fn decode(private: &str) -> Value {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
    let bytes = STANDARD
        .decode(private.trim())
        .or_else(|_| STANDARD_NO_PAD.decode(private.trim().trim_end_matches('=')));
    bytes
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null)
}

pub fn parse_presences(v: &Value) -> Vec<Presence> {
    v.get("presences")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let product = str_at(p, "product").unwrap_or_default();
            let data = if product == "valorant" {
                str_at(p, "private").map(|x| decode(&x)).unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            Some(Presence {
                puuid: str_at(p, "puuid")?,
                product,
                state: str_at(p, "state").unwrap_or_default(),
                data,
            })
        })
        .collect()
}

/// Depth-first search for a key; Riot has moved presence fields into nested
/// objects over time, so look anywhere.
pub fn find<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(map) => {
            if let Some(x) = map.get(key) {
                return Some(x);
            }
            map.values().find_map(|x| find(x, key))
        }
        Value::Array(a) => a.iter().find_map(|x| find(x, key)),
        _ => None,
    }
}

fn find_str(v: &Value, key: &str) -> Option<String> {
    find(v, key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn find_u64(v: &Value, key: &str) -> Option<u64> {
    find(v, key).and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
}

impl Presence {
    pub fn party_id(&self) -> Option<String> {
        find_str(&self.data, "partyId")
    }

    pub fn score(&self) -> Option<[u64; 2]> {
        let us = find_u64(&self.data, "partyOwnerMatchScoreAllyTeam")?;
        let them = find_u64(&self.data, "partyOwnerMatchScoreEnemyTeam")?;
        Some([us, them])
    }

    pub fn loop_state(&self) -> Option<String> {
        find_str(&self.data, "sessionLoopState")
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FriendView {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub note: Option<String>,
    pub online: bool,
    pub in_valorant: bool,
    /// Short human status: Menüde / Ajan seçimi / Maçta / League'de…
    pub status: String,
    pub map: Option<String>,
    pub queue: Option<String>,
    pub party_size: Option<u64>,
    pub score: Option<[u64; 2]>,
    pub rank: Option<TierBadge>,
    pub level: Option<u64>,
}

fn product_label(p: &str) -> String {
    match p {
        "league_of_legends" => "League of Legends'ta".into(),
        "bacon" => "Legends of Runeterra'da".into(),
        "keystone" | "" => "Çevrimiçi".into(),
        other => format!("{other} oynuyor"),
    }
}

pub fn friends(friends: &Value, presences: &[Presence], c: &Content) -> Vec<FriendView> {
    let mut out: Vec<FriendView> = friends
        .get("friends")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let puuid = str_at(f, "puuid")?;
            let pres = presences.iter().find(|p| p.puuid == puuid);
            let in_valorant = pres.map(|p| p.product == "valorant").unwrap_or(false);
            let data = pres.map(|p| &p.data).filter(|_| in_valorant);
            let state = data.and_then(|d| find_str(d, "sessionLoopState"));
            let idle = data
                .and_then(|d| find(d, "isIdle"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let status = match (pres, state.as_deref()) {
                (None, _) => "Çevrimdışı".to_string(),
                (Some(p), _) if p.state == "offline" => "Çevrimdışı".to_string(),
                (Some(_), _) if idle => "AFK".to_string(),
                (Some(_), Some("INGAME")) => "Maçta".to_string(),
                (Some(_), Some("PREGAME")) => "Ajan seçimi".to_string(),
                (Some(_), Some("MENUS")) => "Menüde".to_string(),
                (Some(p), _) if !in_valorant => product_label(&p.product),
                (Some(p), _) if p.state == "away" => "Uzakta".to_string(),
                _ => "Çevrimiçi".to_string(),
            };
            let online = status != "Çevrimdışı";
            let in_match = matches!(state.as_deref(), Some("INGAME"));
            let tier = data.and_then(|d| find_u64(d, "competitiveTier")).filter(|t| *t > 0);
            Some(FriendView {
                puuid,
                name: str_at(f, "game_name").unwrap_or_else(|| "?".into()),
                tag: str_at(f, "game_tag").unwrap_or_default(),
                note: str_at(f, "note"),
                online,
                in_valorant,
                status,
                map: data
                    .filter(|_| in_match)
                    .and_then(|d| find_str(d, "matchMap"))
                    .and_then(|m| c.map_name(&m)),
                queue: data
                    .and_then(|d| find_str(d, "queueId"))
                    .map(|q| queue_label(&q)),
                party_size: data.and_then(|d| find_u64(d, "partySize")),
                score: if in_match {
                    pres.and_then(|p| p.score())
                } else {
                    None
                },
                rank: tier.map(|t| c.rank(t)),
                level: data.and_then(|d| find_u64(d, "accountLevel")),
            })
        })
        .collect();
    out.sort_by_key(|f| {
        let order = match f.status.as_str() {
            "Maçta" => 0,
            "Ajan seçimi" => 1,
            "Menüde" => 2,
            "AFK" | "Uzakta" => 4,
            "Çevrimdışı" => 6,
            _ if f.in_valorant => 3,
            _ => 5,
        };
        (order, f.name.to_lowercase())
    });
    out
}

/// Friends that share a party id are queued together.
pub fn party_groups(presences: &[Presence]) -> Vec<Vec<String>> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for p in presences {
        let Some(id) = p.party_id() else { continue };
        let pos = groups.iter().position(|(g, _)| *g == id);
        match pos {
            Some(i) => groups[i].1.push(p.puuid.clone()),
            None => groups.push((id, vec![p.puuid.clone()])),
        }
    }
    groups.into_iter().map(|(_, m)| m).filter(|m| m.len() > 1).collect()
}
