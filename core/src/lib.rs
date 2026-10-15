//! Reads the current VALORANT match and resolves every player's weapon skins.
//!
//! Hosts contacted: 127.0.0.1 (Riot Client), Riot's game servers
//! (*.a.pvp.net) and valorant-api.com (public skin database). Nothing else.

pub mod content;
pub mod local;
pub mod riot;

use content::Content;
use riot::{str_at, Riot, Session};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const SOCKET_SKIN: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";
const SOCKET_CHROMA: &str = "3ad1b2b2-acdb-4524-852f-954a76ddae0a";

#[derive(Debug, Serialize, Clone)]
pub struct WeaponSkin {
    pub weapon_id: String,
    pub skin: String,
    pub image: Option<String>,
    pub tier_color: Option<String>,
    pub standard: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlayerView {
    pub puuid: String,
    pub name: Option<String>,
    pub tag: Option<String>,
    /// The player turned on Riot's "hide my name"; we respect it.
    pub hidden: bool,
    pub agent: Option<String>,
    pub agent_icon: Option<String>,
    pub ally: bool,
    pub is_me: bool,
    pub level: Option<u64>,
    pub weapons: Vec<WeaponSkin>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WeaponMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Snapshot {
    /// riot_closed | waiting | menu | pregame | ingame | error
    pub status: String,
    pub message: Option<String>,
    pub map: Option<String>,
    pub queue: Option<String>,
    pub players: Vec<PlayerView>,
    pub weapons: Vec<WeaponMeta>,
}

impl Snapshot {
    fn status(status: &str, message: impl Into<String>) -> Self {
        Snapshot {
            status: status.into(),
            message: Some(message.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RawPlayer {
    puuid: String,
    team: String,
    agent_id: Option<String>,
    incognito: bool,
    level: Option<u64>,
}

fn raw_player(p: &Value, team: String) -> Option<RawPlayer> {
    let ident = p.get("PlayerIdentity");
    let flag = |k: &str| {
        ident
            .and_then(|i| i.get(k))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let level = if flag("HideAccountLevel") {
        None
    } else {
        ident.and_then(|i| i.get("AccountLevel")).and_then(Value::as_u64)
    };
    Some(RawPlayer {
        puuid: str_at(p, "Subject")?,
        team,
        agent_id: str_at(p, "CharacterID").map(|x| x.to_lowercase()),
        incognito: flag("Incognito"),
        level,
    })
}

fn players_from_core(m: &Value) -> Vec<RawPlayer> {
    m.get("Players")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| raw_player(p, str_at(p, "TeamID").unwrap_or_default()))
        .collect()
}

fn players_from_pregame(m: &Value) -> Vec<RawPlayer> {
    m.pointer("/AllyTeam/Players")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| raw_player(p, "ally".into()))
        .collect()
}

/// Loadout responses differ slightly between pregame and core-game; accept
/// both and fall back to list order when a loadout has no subject.
fn loadouts_by_player(l: &Value, players: &[RawPlayer]) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    let list = l.get("Loadouts").and_then(Value::as_array);
    for (i, entry) in list.into_iter().flatten().enumerate() {
        let body = entry.get("Loadout").unwrap_or(entry);
        let Some(items) = body.get("Items") else { continue };
        let subject = str_at(body, "Subject")
            .or_else(|| str_at(entry, "Subject"))
            .or_else(|| players.get(i).map(|p| p.puuid.clone()));
        if let Some(s) = subject {
            out.insert(s, items.clone());
        }
    }
    out
}

fn socket_item(item: &Value, socket: &str) -> Option<String> {
    item.get("Sockets")?
        .get(socket)?
        .get("Item")?
        .get("ID")?
        .as_str()
        .map(str::to_lowercase)
}

fn category_rank(cat: &str) -> u8 {
    match cat {
        "Rifle" => 0,
        "Sniper" => 1,
        "SMG" => 2,
        "Shotgun" => 3,
        "Heavy" => 4,
        "Sidearm" => 5,
        "Melee" => 6,
        _ => 7,
    }
}

fn weapon_skins(items: &Value, c: &Content) -> Vec<WeaponSkin> {
    let mut out: Vec<WeaponSkin> = Vec::new();
    for (wid, item) in items.as_object().into_iter().flatten() {
        let wid = wid.to_lowercase();
        let weapon = c.weapons.get(&wid);
        let skin_id = socket_item(item, SOCKET_SKIN);
        let skin = skin_id.as_ref().and_then(|s| c.skins.get(s));
        let chroma = socket_item(item, SOCKET_CHROMA).and_then(|ch| c.chromas.get(&ch));
        let standard = match (&skin_id, weapon.and_then(|w| w.default_skin.as_ref())) {
            (Some(s), Some(d)) => s == d,
            (None, _) => true,
            _ => false,
        };
        let name = chroma
            .map(|ch| ch.name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| skin.map(|s| s.name.clone()))
            .or_else(|| weapon.map(|w| w.name.clone()))
            .unwrap_or_else(|| "Bilinmiyor".into());
        let image = chroma
            .and_then(|ch| ch.icon.clone())
            .or_else(|| skin.and_then(|s| s.icon.clone()))
            .or_else(|| weapon.and_then(|w| w.icon.clone()));
        let tier_color = skin
            .and_then(|s| s.tier.as_ref())
            .and_then(|t| c.tiers.get(t))
            .cloned();
        out.push(WeaponSkin {
            weapon_id: wid,
            skin: name,
            image,
            tier_color,
            standard,
        });
    }
    out.sort_by_key(|w| {
        let meta = c.weapons.get(&w.weapon_id);
        (
            meta.map(|m| category_rank(&m.category)).unwrap_or(9),
            meta.map(|m| m.name.clone()).unwrap_or_default(),
        )
    });
    out
}

fn weapon_meta(players: &[PlayerView], c: &Content) -> Vec<WeaponMeta> {
    let mut seen: Vec<String> = Vec::new();
    for p in players {
        for w in &p.weapons {
            if !seen.contains(&w.weapon_id) {
                seen.push(w.weapon_id.clone());
            }
        }
    }
    let mut out: Vec<WeaponMeta> = seen
        .into_iter()
        .filter_map(|id| {
            let w = c.weapons.get(&id)?;
            Some(WeaponMeta {
                id,
                name: w.name.clone(),
                category: w.category.clone(),
                icon: w.icon.clone(),
            })
        })
        .collect();
    out.sort_by_key(|w| (category_rank(&w.category), w.name.clone()));
    out
}

fn assemble(
    raw: &[RawPlayer],
    loadouts: &HashMap<String, Value>,
    c: &Content,
    me: &str,
    names: &HashMap<String, (String, String)>,
) -> Vec<PlayerView> {
    let my_team = raw
        .iter()
        .find(|p| p.puuid == me)
        .map(|p| p.team.clone())
        .unwrap_or_default();
    let mut out: Vec<PlayerView> = raw
        .iter()
        .map(|p| {
            let is_me = p.puuid == me;
            let hidden = p.incognito && !is_me;
            let name = if hidden { None } else { names.get(&p.puuid) };
            let agent = p.agent_id.as_ref().and_then(|a| c.agents.get(a));
            PlayerView {
                puuid: p.puuid.clone(),
                name: name.map(|n| n.0.clone()),
                tag: name.map(|n| n.1.clone()).filter(|t| !t.is_empty()),
                hidden,
                agent: agent.map(|a| a.name.clone()),
                agent_icon: agent.and_then(|a| a.icon.clone()),
                ally: p.team == my_team,
                is_me,
                level: p.level,
                weapons: loadouts
                    .get(&p.puuid)
                    .map(|items| weapon_skins(items, c))
                    .unwrap_or_default(),
            }
        })
        .collect();
    out.sort_by_key(|p| (!p.ally, !p.is_me));
    out
}

fn queue_label(q: &str) -> String {
    match q {
        "competitive" => "Rekabetçi".into(),
        "unrated" => "Derecesiz".into(),
        "swiftplay" => "Tam Gaz".into(),
        "spikerush" => "Spike'a Hücum".into(),
        "deathmatch" => "Tam Kapışma".into(),
        "hurm" => "Takım Kapışması".into(),
        "ggteam" => "Tırmanış".into(),
        "premier" => "Premier".into(),
        "" => "Özel oyun".into(),
        other => {
            let mut c = other.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
}

pub struct Engine {
    riot: Riot,
    content: tokio::sync::Mutex<Option<Arc<Content>>>,
    names: Mutex<HashMap<String, (String, String)>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            riot: Riot::new(),
            content: tokio::sync::Mutex::new(None),
            names: Mutex::new(HashMap::new()),
        }
    }

    async fn content(&self) -> Result<Arc<Content>, String> {
        let mut slot = self.content.lock().await;
        if let Some(c) = slot.as_ref() {
            return Ok(c.clone());
        }
        let c = Arc::new(Content::download(self.riot.remote()).await?);
        *slot = Some(c.clone());
        Ok(c)
    }

    async fn resolve_names(&self, s: &Session, raw: &[RawPlayer]) -> HashMap<String, (String, String)> {
        let missing: Vec<String> = {
            let cache = self.names.lock().unwrap();
            raw.iter()
                .filter(|p| !p.incognito || p.puuid == s.puuid)
                .map(|p| p.puuid.clone())
                .filter(|id| !cache.contains_key(id))
                .collect()
        };
        if !missing.is_empty() {
            if let Ok(found) = self.riot.names(s, &missing).await {
                let mut cache = self.names.lock().unwrap();
                for (id, name, tag) in found {
                    cache.insert(id, (name, tag));
                }
            }
        }
        self.names.lock().unwrap().clone()
    }

    async fn build(&self, s: &Session, c: &Content, phase: &str, m: &Value, raw: Vec<RawPlayer>, loadouts: &Value) -> Snapshot {
        let by_player = loadouts_by_player(loadouts, &raw);
        let names = self.resolve_names(s, &raw).await;
        let players = assemble(&raw, &by_player, c, &s.puuid, &names);
        let queue = str_at(m, "QueueID")
            .or_else(|| m.pointer("/MatchmakingData/QueueID").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        Snapshot {
            status: phase.into(),
            message: None,
            map: str_at(m, "MapID").and_then(|id| c.maps.get(&id).cloned()),
            queue: Some(queue_label(&queue)),
            weapons: weapon_meta(&players, c),
            players,
        }
    }

    pub async fn snapshot(&self) -> Snapshot {
        let Some(lf) = local::read_lockfile() else {
            return Snapshot::status("riot_closed", "Riot Client açık değil.");
        };
        let log = local::read_log();
        let s = match self.riot.session(&lf, &log).await {
            Ok(s) => s,
            Err(e) => return Snapshot::status("waiting", e),
        };
        let c = match self.content().await {
            Ok(c) => c,
            Err(e) => return Snapshot::status("error", format!("Skin verileri indirilemedi: {e}")),
        };

        // In a match (or loading into one).
        match self.riot.glz(&s, &format!("/core-game/v1/players/{}", s.puuid)).await {
            Ok(Some(v)) => {
                if let Some(id) = str_at(&v, "MatchID") {
                    let m = self.riot.glz(&s, &format!("/core-game/v1/matches/{id}")).await;
                    let l = self.riot.glz(&s, &format!("/core-game/v1/matches/{id}/loadouts")).await;
                    if let (Ok(Some(m)), Ok(Some(l))) = (m, l) {
                        let raw = players_from_core(&m);
                        return self.build(&s, &c, "ingame", &m, raw, &l).await;
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Snapshot::status("error", e),
        }

        // Agent select.
        match self.riot.glz(&s, &format!("/pregame/v1/players/{}", s.puuid)).await {
            Ok(Some(v)) => {
                if let Some(id) = str_at(&v, "MatchID") {
                    let m = self.riot.glz(&s, &format!("/pregame/v1/matches/{id}")).await;
                    let l = self.riot.glz(&s, &format!("/pregame/v1/matches/{id}/loadouts")).await;
                    if let Ok(Some(m)) = m {
                        let raw = players_from_pregame(&m);
                        let l = l.ok().flatten().unwrap_or(Value::Null);
                        return self.build(&s, &c, "pregame", &m, raw, &l).await;
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Snapshot::status("error", e),
        }

        Snapshot::status("menu", "Maç bekleniyor. Ajan seçimine girdiğinde takımının skinleri, maç başlayınca herkesinki görünür.")
    }
}

#[cfg(test)]
mod tests;
