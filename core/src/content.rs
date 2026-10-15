//! Names and pictures for agents, weapons, skins, ranks and more, from
//! valorant-api.com. Downloaded once per launch.

use serde::Serialize;
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
    pub weapon: String,
}

#[derive(Debug, Clone)]
pub struct Chroma {
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub small: Option<String>,
    pub wide: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Border {
    pub id: String,
    pub start: u64,
    pub image: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MissionDef {
    pub title: String,
    /// daily | weekly | other
    pub kind: String,
    pub xp: u64,
    pub target: u64,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct TierBadge {
    pub tier: u64,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub levels: u64,
    /// Season (act) uuid for battle passes.
    pub season: Option<String>,
}

#[derive(Debug, Default)]
pub struct Content {
    pub agents: HashMap<String, Agent>,
    pub weapons: HashMap<String, Weapon>,
    pub skins: HashMap<String, Skin>,
    /// skin level uuid -> skin uuid (store and inventory use level ids)
    pub levels: HashMap<String, String>,
    pub chromas: HashMap<String, Chroma>,
    /// content tier uuid -> highlight colour (#rrggbb)
    pub tiers: HashMap<String, String>,
    /// content tier uuid -> rank used for sorting (higher = rarer)
    pub tier_rank: HashMap<String, u64>,
    /// map path (/Game/Maps/...) -> display name
    pub maps: HashMap<String, String>,
    /// buddy level uuid -> icon
    pub buddies: HashMap<String, String>,
    /// buddy level uuid -> name
    pub buddy_names: HashMap<String, String>,
    /// level borders, lowest level first
    pub borders: Vec<Border>,
    pub missions: HashMap<String, MissionDef>,
    pub cards: HashMap<String, Card>,
    pub titles: HashMap<String, String>,
    pub ranks: HashMap<u64, TierBadge>,
    pub contracts: HashMap<String, Contract>,
    /// bundle uuid -> (name, image)
    pub bundles: HashMap<String, (String, Option<String>)>,
    /// spray / flex uuid (and spray level uuids) -> (name, icon)
    pub cosmetics: HashMap<String, (String, Option<String>)>,
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

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

fn items(v: &Value) -> impl Iterator<Item = &Value> {
    arr(v, "data")
}

/// valorant-api colours are RRGGBBAA.
fn colour(raw: &str) -> String {
    let hex: String = raw.chars().take(6).collect();
    format!("#{hex}")
}

#[derive(Default)]
pub struct Sources {
    pub agents: Value,
    pub weapons: Value,
    pub tiers: Value,
    pub maps: Value,
    pub buddies: Value,
    pub cards: Value,
    pub titles: Value,
    pub ranks: Value,
    pub contracts: Value,
    pub bundles: Value,
    pub sprays: Value,
    pub flex: Value,
    pub borders: Value,
    pub missions: Value,
}

impl Content {
    pub fn from_sources(src: &Sources) -> Self {
        let mut c = Content::default();

        for a in items(&src.agents) {
            if let (Some(uuid), Some(name)) = (id(a, "uuid"), s(a, "displayName")) {
                let icon = s(a, "displayIconSmall").or_else(|| s(a, "displayIcon"));
                c.agents.insert(uuid, Agent { name, icon });
            }
        }

        for w in items(&src.weapons) {
            let Some(wid) = id(w, "uuid") else { continue };
            let category = s(w, "category")
                .map(|x| x.rsplit("::").next().unwrap_or(x.as_str()).to_string())
                .unwrap_or_default();
            c.weapons.insert(
                wid.clone(),
                Weapon {
                    name: s(w, "displayName").unwrap_or_default(),
                    category,
                    icon: s(w, "displayIcon"),
                    default_skin: id(w, "defaultSkinUuid"),
                },
            );
            for sk in arr(w, "skins") {
                let Some(sid) = id(sk, "uuid") else { continue };
                let first_level_icon = sk
                    .pointer("/levels/0/displayIcon")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                c.skins.insert(
                    sid.clone(),
                    Skin {
                        name: s(sk, "displayName").unwrap_or_default(),
                        icon: s(sk, "displayIcon").or(first_level_icon),
                        tier: id(sk, "contentTierUuid"),
                        weapon: wid.clone(),
                    },
                );
                for lv in arr(sk, "levels") {
                    if let Some(lid) = id(lv, "uuid") {
                        c.levels.insert(lid, sid.clone());
                    }
                }
                for ch in arr(sk, "chromas") {
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

        for t in items(&src.tiers) {
            if let Some(uuid) = id(t, "uuid") {
                if let Some(col) = s(t, "highlightColor") {
                    c.tiers.insert(uuid.clone(), colour(&col));
                }
                if let Some(r) = t.get("rank").and_then(Value::as_u64) {
                    c.tier_rank.insert(uuid, r);
                }
            }
        }

        for m in items(&src.maps) {
            if let (Some(url), Some(name)) = (s(m, "mapUrl"), s(m, "displayName")) {
                c.maps.insert(url, name);
            }
        }

        for b in items(&src.buddies) {
            let icon = s(b, "displayIcon");
            let name = s(b, "displayName").unwrap_or_default();
            for lv in arr(b, "levels") {
                let Some(lid) = id(lv, "uuid") else { continue };
                if let Some(i) = s(lv, "displayIcon").or(icon.clone()) {
                    c.buddies.insert(lid.clone(), i);
                }
                c.buddy_names.insert(lid, name.clone());
            }
        }

        for b in items(&src.borders) {
            if let Some(uuid) = id(b, "uuid") {
                c.borders.push(Border {
                    id: uuid,
                    start: b.get("startingLevel").and_then(Value::as_u64).unwrap_or(0),
                    image: s(b, "levelNumberAppearance").or_else(|| s(b, "smallPlayerCardAppearance")),
                });
            }
        }
        c.borders.sort_by_key(|b| b.start);

        for m in items(&src.missions) {
            let Some(uuid) = id(m, "uuid") else { continue };
            let kind = match s(m, "type").as_deref().map(|t| t.rsplit("::").next().unwrap_or(t)) {
                Some(t) if t.eq_ignore_ascii_case("Daily") => "daily",
                Some(t) if t.eq_ignore_ascii_case("Weekly") => "weekly",
                _ => "other",
            };
            c.missions.insert(
                uuid,
                MissionDef {
                    title: s(m, "title").or_else(|| s(m, "displayName")).unwrap_or_else(|| "Görev".into()),
                    kind: kind.into(),
                    xp: m.get("xpGrant").and_then(Value::as_u64).unwrap_or(0),
                    target: m.get("progressToComplete").and_then(Value::as_u64).unwrap_or(0),
                },
            );
        }

        for card in items(&src.cards) {
            if let Some(uuid) = id(card, "uuid") {
                c.cards.insert(
                    uuid,
                    Card {
                        name: s(card, "displayName").unwrap_or_default(),
                        small: s(card, "smallArt").or_else(|| s(card, "displayIcon")),
                        wide: s(card, "wideArt"),
                    },
                );
            }
        }

        for t in items(&src.titles) {
            if let (Some(uuid), Some(text)) = (id(t, "uuid"), s(t, "titleText")) {
                c.titles.insert(uuid, text);
            }
        }

        // The last table is the current one.
        if let Some(table) = src.ranks.get("data").and_then(Value::as_array).and_then(|a| a.last()) {
            for t in arr(table, "tiers") {
                let Some(tier) = t.get("tier").and_then(Value::as_u64) else { continue };
                c.ranks.insert(
                    tier,
                    TierBadge {
                        tier,
                        name: s(t, "tierName").unwrap_or_default(),
                        color: s(t, "color").map(|x| colour(&x)),
                        icon: s(t, "smallIcon").or_else(|| s(t, "largeIcon")),
                    },
                );
            }
        }

        for ct in items(&src.contracts) {
            let Some(uuid) = id(ct, "uuid") else { continue };
            let levels: usize = ct
                .pointer("/content/chapters")
                .and_then(Value::as_array)
                .map(|chs| chs.iter().map(|ch| arr(ch, "levels").count()).sum())
                .unwrap_or(0);
            let season = match ct.pointer("/content/relationType").and_then(Value::as_str) {
                Some("Season") => ct
                    .pointer("/content/relationUuid")
                    .and_then(Value::as_str)
                    .map(str::to_lowercase),
                _ => None,
            };
            c.contracts.insert(
                uuid,
                Contract {
                    name: s(ct, "displayName").unwrap_or_default(),
                    levels: levels as u64,
                    season,
                },
            );
        }

        for sp in items(&src.sprays) {
            let Some(name) = s(sp, "displayName") else { continue };
            let icon = s(sp, "fullTransparentIcon")
                .or_else(|| s(sp, "fullIcon"))
                .or_else(|| s(sp, "displayIcon"));
            if let Some(uuid) = id(sp, "uuid") {
                c.cosmetics.insert(uuid, (name.clone(), icon.clone()));
            }
            for lv in arr(sp, "levels") {
                if let Some(lid) = id(lv, "uuid") {
                    c.cosmetics.insert(lid, (name.clone(), icon.clone()));
                }
            }
        }
        for f in items(&src.flex) {
            if let (Some(uuid), Some(name)) = (id(f, "uuid"), s(f, "displayName")) {
                c.cosmetics.insert(uuid, (name, s(f, "displayIcon")));
            }
        }

        for b in items(&src.bundles) {
            if let (Some(uuid), Some(name)) = (id(b, "uuid"), s(b, "displayName")) {
                let icon = s(b, "displayIcon2").or_else(|| s(b, "displayIcon"));
                c.bundles.insert(uuid, (name, icon));
            }
        }
        c
    }

    pub async fn download(http: &reqwest::Client) -> Result<Self, String> {
        async fn get(http: &reqwest::Client, path: &str) -> Result<Value, String> {
            let sep = if path.contains('?') { '&' } else { '?' };
            let url = format!("https://valorant-api.com/v1/{path}{sep}language={LANG}");
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
        // Optional extras must not stop skins from working.
        async fn opt(http: &reqwest::Client, path: &str) -> Value {
            get(http, path).await.unwrap_or(Value::Null)
        }
        let src = Sources {
            agents: get(http, "agents?isPlayableCharacter=true").await?,
            weapons: get(http, "weapons").await?,
            tiers: opt(http, "contenttiers").await,
            maps: opt(http, "maps").await,
            buddies: opt(http, "buddies").await,
            cards: opt(http, "playercards").await,
            titles: opt(http, "playertitles").await,
            ranks: opt(http, "competitivetiers").await,
            contracts: opt(http, "contracts").await,
            bundles: opt(http, "bundles").await,
            sprays: opt(http, "sprays").await,
            flex: opt(http, "flex").await,
            borders: opt(http, "levelborders").await,
            missions: opt(http, "missions").await,
        };
        let c = Self::from_sources(&src);
        if c.weapons.is_empty() {
            return Err("Silah listesi boş geldi".into());
        }
        Ok(c)
    }

    pub fn rank(&self, tier: u64) -> TierBadge {
        self.ranks.get(&tier).cloned().unwrap_or(TierBadge {
            tier,
            name: if tier == 0 { "Derecesiz".into() } else { format!("Seviye {tier}") },
            color: None,
            icon: None,
        })
    }

    /// Border picture: the one the player picked, else the one their level earns.
    pub fn border(&self, id: Option<&str>, level: Option<u64>) -> Option<String> {
        if let Some(id) = id {
            if let Some(b) = self.borders.iter().find(|b| b.id.eq_ignore_ascii_case(id)) {
                return b.image.clone();
            }
        }
        let level = level?;
        self.borders
            .iter()
            .filter(|b| b.start <= level)
            .last()
            .and_then(|b| b.image.clone())
    }

    pub fn map_name(&self, path: &str) -> Option<String> {
        self.maps.get(path).cloned()
    }
}
