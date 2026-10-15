//! Turns Riot's match and loadout data into what the window shows.

use crate::content::{Content, TierBadge};
use crate::history::Encounter;
use crate::ranks::RankInfo;
use crate::riot::{str_at, u64_at};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub const SOCKET_SKIN: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";
pub const SOCKET_CHROMA: &str = "3ad1b2b2-acdb-4524-852f-954a76ddae0a";
pub const SOCKET_BUDDY: &str = "77258665-71d1-4623-bc72-44db9bd5b3b3";

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct WeaponSkin {
    pub weapon_id: String,
    pub skin: String,
    pub image: Option<String>,
    pub tier_color: Option<String>,
    pub standard: bool,
    pub buddy: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
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
    /// Level border picture.
    pub border: Option<String>,
    pub card: Option<String>,
    pub title: Option<String>,
    pub rank: Option<RankInfo>,
    /// Same number = queued together. Numbered from 1 per match.
    pub party: Option<u32>,
    /// Met in one of your recent matches (advanced only).
    pub encounter: Option<Encounter>,
    pub weapons: Vec<WeaponSkin>,
    /// Equipped sprays and flex items.
    pub sprays: Vec<Cosmetic>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Cosmetic {
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct WeaponMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
pub struct Snapshot {
    /// riot_closed | waiting | menu | pregame | ingame | error
    pub status: String,
    pub message: Option<String>,
    pub match_id: Option<String>,
    pub map: Option<String>,
    pub queue: Option<String>,
    pub score: Option<[u64; 2]>,
    /// Whether premade groups and peak ranks are being shown.
    pub advanced: bool,
    /// Your party while in menus.
    pub lobby: Option<Lobby>,
    pub players: Vec<PlayerView>,
    pub weapons: Vec<WeaponMeta>,
}

impl Snapshot {
    pub fn status(status: &str, message: impl Into<String>) -> Self {
        Snapshot {
            status: status.into(),
            message: Some(message.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawPlayer {
    pub puuid: String,
    pub team: String,
    pub agent_id: Option<String>,
    pub incognito: bool,
    pub level: Option<u64>,
    pub card_id: Option<String>,
    pub title_id: Option<String>,
    pub border_id: Option<String>,
}

fn raw_player(p: &Value, team: String) -> Option<RawPlayer> {
    let ident = p.get("PlayerIdentity");
    let flag = |k: &str| {
        ident
            .and_then(|i| i.get(k))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let ident_str = |k: &str| ident.and_then(|i| str_at(i, k)).map(|x| x.to_lowercase());
    let level = if flag("HideAccountLevel") {
        None
    } else {
        ident.and_then(|i| u64_at(i, "AccountLevel"))
    };
    Some(RawPlayer {
        puuid: str_at(p, "Subject")?,
        team,
        agent_id: str_at(p, "CharacterID").map(|x| x.to_lowercase()),
        incognito: flag("Incognito"),
        level,
        card_id: ident_str("PlayerCardID"),
        title_id: ident_str("PlayerTitleID"),
        border_id: ident_str("PreferredLevelBorderID"),
    })
}

pub fn players_from_core(m: &Value) -> Vec<RawPlayer> {
    m.get("Players")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| raw_player(p, str_at(p, "TeamID").unwrap_or_default()))
        .collect()
}

pub fn players_from_pregame(m: &Value) -> Vec<RawPlayer> {
    m.pointer("/AllyTeam/Players")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| raw_player(p, "ally".into()))
        .collect()
}

/// Loadout responses differ slightly between pregame and core-game; accept
/// both and fall back to list order when a loadout has no subject.
pub fn loadouts_by_player(l: &Value, players: &[RawPlayer]) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    let list = l.get("Loadouts").and_then(Value::as_array);
    for (i, entry) in list.into_iter().flatten().enumerate() {
        let body = entry.get("Loadout").unwrap_or(entry);
        if body.get("Items").is_none() {
            continue;
        }
        let subject = str_at(body, "Subject")
            .or_else(|| str_at(entry, "Subject"))
            .or_else(|| players.get(i).map(|p| p.puuid.clone()));
        if let Some(s) = subject {
            out.insert(s, body.clone());
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

pub fn category_rank(cat: &str) -> u8 {
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

pub fn weapon_skins(items: &Value, c: &Content) -> Vec<WeaponSkin> {
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
        let buddy = socket_item(item, SOCKET_BUDDY).and_then(|b| c.buddies.get(&b).cloned());
        out.push(WeaponSkin {
            weapon_id: wid,
            skin: name,
            image,
            tier_color,
            standard,
            buddy,
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

/// Sprays and flex from a loadout. Riot has used both a `Sprays` block and a
/// newer `Expressions` block, so read whichever is there.
pub fn cosmetics(body: &Value, c: &Content) -> Vec<Cosmetic> {
    let mut ids: Vec<String> = Vec::new();
    let mut push = |v: Option<&Value>| {
        if let Some(id) = v.and_then(Value::as_str).map(str::to_lowercase) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    };
    let list = |v: &Value, a: &str, b: &str| -> Vec<Value> {
        v.get(a)
            .and_then(|x| x.get(b))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for s in list(body, "Sprays", "SpraySelections") {
        push(s.get("SprayID"));
    }
    for s in list(body, "Expressions", "AESSelections") {
        push(s.get("AssetID"));
    }
    ids.iter()
        .filter_map(|id| c.cosmetics.get(id))
        .map(|(name, icon)| Cosmetic {
            name: name.clone(),
            icon: icon.clone(),
        })
        .collect()
}

pub fn weapon_meta(players: &[PlayerView], c: &Content) -> Vec<WeaponMeta> {
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

/// Merge overlapping groups and number the ones with two or more players,
/// in roster order so the numbering is stable between polls.
pub fn number_parties(groups: &[Vec<String>], roster: &[String]) -> HashMap<String, u32> {
    let mut root: HashMap<String, String> = HashMap::new();
    fn find(root: &mut HashMap<String, String>, x: &str) -> String {
        let mut cur = x.to_string();
        while let Some(next) = root.get(&cur) {
            if *next == cur {
                break;
            }
            cur = next.clone();
        }
        cur
    }
    for g in groups {
        let members: Vec<&String> = g.iter().filter(|p| roster.contains(p)).collect();
        for pair in members.windows(2) {
            let a = find(&mut root, pair[0]);
            let b = find(&mut root, pair[1]);
            if a != b {
                root.insert(b, a.clone());
                root.insert(a.clone(), a);
            }
        }
    }
    let mut size: HashMap<String, u32> = HashMap::new();
    for p in roster {
        *size.entry(find(&mut root, p)).or_default() += 1;
    }
    let mut numbers: HashMap<String, u32> = HashMap::new();
    let mut out = HashMap::new();
    for p in roster {
        let r = find(&mut root, p);
        if size.get(&r).copied().unwrap_or(0) < 2 {
            continue;
        }
        let next = numbers.len() as u32 + 1;
        let n = *numbers.entry(r).or_insert(next);
        out.insert(p.clone(), n);
    }
    out
}

pub struct Extras<'a> {
    pub names: &'a HashMap<String, (String, String)>,
    pub ranks: &'a HashMap<String, RankInfo>,
    pub parties: &'a HashMap<String, u32>,
    pub encounters: &'a HashMap<String, Encounter>,
}

pub fn assemble(
    raw: &[RawPlayer],
    loadouts: &HashMap<String, Value>,
    c: &Content,
    me: &str,
    x: &Extras,
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
            let name = if hidden { None } else { x.names.get(&p.puuid) };
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
                border: c.border(p.border_id.as_deref(), p.level),
                card: p.card_id.as_ref().and_then(|id| c.cards.get(id)).and_then(|k| k.small.clone()),
                title: p.title_id.as_ref().and_then(|id| c.titles.get(id)).cloned(),
                rank: x.ranks.get(&p.puuid).cloned(),
                party: x.parties.get(&p.puuid).copied(),
                encounter: if is_me { None } else { x.encounters.get(&p.puuid).cloned() },
                weapons: loadouts
                    .get(&p.puuid)
                    .and_then(|body| body.get("Items"))
                    .map(|items| weapon_skins(items, c))
                    .unwrap_or_default(),
                sprays: loadouts
                    .get(&p.puuid)
                    .map(|body| cosmetics(body, c))
                    .unwrap_or_default(),
            }
        })
        .collect();
    out.sort_by_key(|p| (!p.ally, !p.is_me, p.party.unwrap_or(u32::MAX)));
    out
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct LobbyMember {
    pub puuid: String,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub is_me: bool,
    pub owner: bool,
    pub ready: bool,
    pub level: Option<u64>,
    pub border: Option<String>,
    pub card: Option<String>,
    pub title: Option<String>,
    pub rank: TierBadge,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Lobby {
    pub queue: String,
    pub searching: bool,
    pub members: Vec<LobbyMember>,
}

/// Your party from `/parties/v1/parties/{id}`.
pub fn lobby(party: &Value, me: &str, c: &Content, names: &HashMap<String, (String, String)>) -> Lobby {
    let mut members: Vec<LobbyMember> = party
        .get("Members")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let raw = raw_player(m, String::new())?;
            let name = names.get(&raw.puuid);
            Some(LobbyMember {
                is_me: raw.puuid == me,
                owner: m.get("IsOwner").and_then(Value::as_bool).unwrap_or(false),
                ready: m.get("IsReady").and_then(Value::as_bool).unwrap_or(false),
                name: name.map(|n| n.0.clone()),
                tag: name.map(|n| n.1.clone()).filter(|t| !t.is_empty()),
                border: c.border(raw.border_id.as_deref(), raw.level),
                card: raw.card_id.as_ref().and_then(|id| c.cards.get(id)).and_then(|k| k.small.clone()),
                title: raw.title_id.as_ref().and_then(|id| c.titles.get(id)).cloned(),
                rank: c.rank(u64_at(m, "CompetitiveTier").unwrap_or(0)),
                level: raw.level,
                puuid: raw.puuid,
            })
        })
        .collect();
    members.sort_by_key(|m| (!m.is_me, !m.owner));
    Lobby {
        queue: crate::stats::queue_label(
            &m_str(party, "/MatchmakingData/QueueID").unwrap_or_default(),
        ),
        searching: str_at(party, "State").as_deref() == Some("MATCHMAKING"),
        members,
    }
}

fn m_str(v: &Value, ptr: &str) -> Option<String> {
    v.pointer(ptr).and_then(Value::as_str).map(str::to_string)
}
