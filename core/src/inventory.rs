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

/// Standard VP price per content tier (weapon, melee), used when Riot's
/// price list is not available. Special collections can cost more.
const TIER_PRICES: [(&str, u64, u64); 5] = [
    ("12683d76-48d7-84a3-4e09-6985794f0445", 875, 1750),  // Select
    ("0cebb8be-46d7-c12a-d306-e9907bfc5a25", 1275, 2550), // Deluxe
    ("60bca009-4182-7998-dee7-b8a2558dc369", 1775, 3550), // Premium
    ("e046854e-406c-37f4-6607-19a9ba8426fc", 2175, 4350), // Exclusive
    ("411e4a55-4e59-7757-41f0-86a53f101bb5", 2475, 4950), // Ultra
];

fn tier_price(tier: &str, melee: bool) -> Option<u64> {
    TIER_PRICES
        .iter()
        .find(|(id, _, _)| id.eq_ignore_ascii_case(tier))
        .map(|(_, gun, knife)| if melee { *knife } else { *gun })
}

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

/// Item id (lower-case) -> VP price, from every offer found in a store
/// response. Riot removed the full price list, so this collects whatever the
/// storefront carries: daily offers, night market, bundle items, accessory and
/// plugin stores. Works on the old `{"Offers": [...]}` shape too.
pub fn prices(v: &Value) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    collect(v, &mut out);
    out
}

fn collect(v: &Value, out: &mut HashMap<String, u64>) {
    match v {
        Value::Object(map) => {
            // { "Cost": {VP: n}, "Rewards": [{ "ItemID": .. }] }
            if let (Some(cost), Some(rewards)) = (
                map.get("Cost").and_then(|c| u64_at(c, VP)),
                map.get("Rewards").and_then(Value::as_array),
            ) {
                // A reward bundle (several items for one price) says nothing
                // about each item, so only single-item offers count.
                if rewards.len() == 1 {
                    if let Some(id) = str_at(&rewards[0], "ItemID") {
                        out.entry(id.to_lowercase()).or_insert(cost);
                    }
                }
            }
            // Bundle items: { "Item": { "ItemID": .. }, "BasePrice": n, "CurrencyID": VP }
            if let (Some(item), Some(price)) = (map.get("Item"), map.get("BasePrice").and_then(Value::as_u64)) {
                let vp = map
                    .get("CurrencyID")
                    .and_then(Value::as_str)
                    .map(|c| c.eq_ignore_ascii_case(VP))
                    .unwrap_or(true);
                if vp && price > 0 {
                    if let Some(id) = str_at(item, "ItemID") {
                        out.entry(id.to_lowercase()).or_insert(price);
                    }
                }
            }
            for x in map.values() {
                collect(x, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect(x, out);
            }
        }
        _ => {}
    }
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
    /// Price guessed from the skin's tier rather than read from the store.
    pub estimated: bool,
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
    /// True when skin prices come from tier estimates.
    pub estimated: bool,
    /// Why the store price list could not be used, if it could not.
    pub note: Option<String>,
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

/// `estimate`: fill in skin prices from their tier when the store gave none.
pub fn build(
    owned: &HashMap<String, Vec<String>>,
    prices: &HashMap<String, u64>,
    c: &Content,
    estimate: bool,
) -> Inventory {
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
        let guess = if estimate {
            let melee = weapon.map(|w| w.category == "Melee").unwrap_or(false);
            skin.tier.as_deref().and_then(|t| tier_price(t, melee))
        } else {
            None
        };
        skins.push(Item {
            id: skin_id.clone(),
            name: skin.name.clone(),
            sub: weapon.map(|w| w.name.clone()),
            image: skin.icon.clone(),
            tier_color: skin.tier.as_ref().and_then(|t| c.tiers.get(t)).cloned(),
            price: best.get(skin_id).copied().or(guess),
            estimated: !best.contains_key(skin_id) && guess.is_some(),
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
                    estimated: false,
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
        estimated: groups.iter().any(|g| g.items.iter().any(|i| i.estimated)),
        note: None,
        groups,
    }
}
