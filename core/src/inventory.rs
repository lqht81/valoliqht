//! Everything your account owns, and roughly what it cost in VP.

use crate::account::VP;
use crate::content::Content;
use crate::riot::{str_at, u64_at};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const T_SKIN: &str = "e7c63390-eda7-46e0-bb7a-a6abdacd2433";
pub const T_BUDDY: &str = "dd3bf334-87f3-40bd-b043-682a57a8dc3a";
pub const T_CARD: &str = "3f296c07-64c3-494c-923b-fe692a4fa1bd";
pub const T_SPRAY: &str = "d5f120f8-ff8c-4aac-92ea-f2b5acbe9475";
pub const T_TITLE: &str = "de7caa6b-adf7-4588-bbd1-143831e786c6";
pub const T_FLEX: &str = "03a572de-4234-31ed-d344-ababa488f981";
pub const T_AGENT: &str = "01bb38e1-da47-4e6a-9b3d-945fe4655707";

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

/// Item id (lower-case) -> VP price, from the store's full offer list.
pub fn prices(offers: &Value) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    for o in arr(offers, "Offers") {
        let Some(price) = o.get("Cost").and_then(|c| u64_at(c, VP)) else { continue };
        for r in arr(o, "Rewards") {
            if let Some(id) = str_at(r, "ItemID") {
                out.insert(id.to_lowercase(), price);
            }
        }
    }
    out
}

/// Entitlements grouped by item type, from `/store/v1/entitlements/{puuid}`.
pub fn by_type(v: &Value) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for t in arr(v, "EntitlementsByTypes") {
        let Some(kind) = str_at(t, "ItemTypeID") else { continue };
        let ids = out.entry(kind.to_lowercase()).or_default();
        for e in arr(t, "Entitlements") {
            if let Some(id) = str_at(e, "ItemID") {
                ids.push(id.to_lowercase());
            }
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub sub: Option<String>,
    pub image: Option<String>,
    pub tier_color: Option<String>,
    pub price: Option<u64>,
    #[serde(skip)]
    pub rarity: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Group {
    pub key: String,
    pub label: String,
    pub items: Vec<Item>,
    pub value: u64,
    pub priced: u32,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Inventory {
    pub groups: Vec<Group>,
    pub agents: u32,
    pub value: u64,
    pub priced: u32,
    pub unpriced: u32,
}

fn group(key: &str, label: &str, mut items: Vec<Item>) -> Group {
    items.sort_by(|a, b| {
        b.rarity
            .cmp(&a.rarity)
            .then(b.price.unwrap_or(0).cmp(&a.price.unwrap_or(0)))
            .then(a.sub.cmp(&b.sub))
            .then(a.name.cmp(&b.name))
    });
    Group {
        key: key.into(),
        label: label.into(),
        value: items.iter().filter_map(|i| i.price).sum(),
        priced: items.iter().filter(|i| i.price.is_some()).count() as u32,
        items,
    }
}

pub fn build(owned: &HashMap<String, Vec<String>>, prices: &HashMap<String, u64>, c: &Content) -> Inventory {
    let get = |t: &str| owned.get(t).cloned().unwrap_or_default();
    let price = |id: &str| prices.get(id).copied();

    // Skins: entitlements are per level, collapse to one per skin.
    let mut skins: Vec<Item> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut best: HashMap<String, u64> = HashMap::new();
    for level in get(T_SKIN) {
        let Some(skin_id) = c.levels.get(&level) else { continue };
        if let Some(p) = price(&level) {
            let e = best.entry(skin_id.clone()).or_insert(0);
            *e = (*e).max(p);
        }
        seen.insert(skin_id.clone());
    }
    for skin_id in &seen {
        let Some(skin) = c.skins.get(skin_id) else { continue };
        let weapon = c.weapons.get(&skin.weapon);
        if weapon.and_then(|w| w.default_skin.as_ref()) == Some(skin_id) {
            continue;
        }
        skins.push(Item {
            id: skin_id.clone(),
            name: skin.name.clone(),
            sub: weapon.map(|w| w.name.clone()),
            image: skin.icon.clone(),
            tier_color: skin.tier.as_ref().and_then(|t| c.tiers.get(t)).cloned(),
            price: best.get(skin_id).copied(),
            rarity: skin.tier.as_ref().and_then(|t| c.tier_rank.get(t)).copied().unwrap_or(0),
        });
    }

    let simple = |t: &str, name: &dyn Fn(&str) -> Option<(String, Option<String>)>| -> Vec<Item> {
        let mut list = Vec::new();
        let mut once = HashSet::new();
        for id in get(t) {
            if !once.insert(id.clone()) {
                continue;
            }
            if let Some((n, img)) = name(&id) {
                list.push(Item {
                    id: id.clone(),
                    name: n,
                    sub: None,
                    image: img,
                    tier_color: None,
                    price: price(&id),
                    rarity: 0,
                });
            }
        }
        list
    };

    let buddies = simple(T_BUDDY, &|id| {
        c.buddy_names
            .get(id)
            .map(|n| (n.clone(), c.buddies.get(id).cloned()))
    });
    let cards = simple(T_CARD, &|id| c.cards.get(id).map(|k| (k.name.clone(), k.small.clone())));
    let sprays = simple(T_SPRAY, &|id| c.cosmetics.get(id).cloned());
    let flex = simple(T_FLEX, &|id| c.cosmetics.get(id).cloned());
    let titles = simple(T_TITLE, &|id| c.titles.get(id).map(|t| (t.clone(), None)));

    let groups = vec![
        group("skins", "Skinler", skins),
        group("buddies", "Buddy'ler", buddies),
        group("cards", "Oyuncu kartları", cards),
        group("sprays", "Spreyler", sprays),
        group("flex", "Flex", flex),
        group("titles", "Unvanlar", titles),
    ];
    let value = groups.iter().map(|g| g.value).sum();
    let priced = groups.iter().map(|g| g.priced).sum();
    let total: u32 = groups.iter().map(|g| g.items.len() as u32).sum();
    Inventory {
        agents: get(T_AGENT).iter().filter(|a| c.agents.contains_key(*a)).count() as u32,
        value,
        priced,
        unpriced: total - priced,
        groups,
    }
}
