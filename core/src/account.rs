//! Your own account: wallet, store, collection, contracts, RR history.

use crate::content::{Content, TierBadge};
use crate::ranks::RankInfo;
use crate::riot::{str_at, u64_at};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

pub const VP: &str = "85ad13f7-3d1b-5128-9eb2-7cd8ee0b5741";
pub const RP: &str = "e59aa87c-4cbf-517a-5983-6e81511be9b7";
pub const KC: &str = "85ca954a-41f2-ce94-9b45-8ca3dd39a00d";
pub const SKIN_LEVEL_TYPE: &str = "e7c63390-eda7-46e0-bb7a-a6abdacd2433";

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Wallet {
    pub vp: Option<u64>,
    pub rp: Option<u64>,
    pub kc: Option<u64>,
}

pub fn wallet(v: &Value) -> Wallet {
    let b = v.get("Balances").cloned().unwrap_or(Value::Null);
    Wallet {
        vp: u64_at(&b, VP),
        rp: u64_at(&b, RP),
        kc: u64_at(&b, KC),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Offer {
    /// Skin uuid, for the wish list.
    pub skin_id: Option<String>,
    pub name: String,
    pub image: Option<String>,
    pub tier_color: Option<String>,
    pub price: Option<u64>,
    pub original: Option<u64>,
    pub discount: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Bundle {
    pub name: String,
    pub image: Option<String>,
    pub ends_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Store {
    pub daily: Vec<Offer>,
    pub daily_ends_in: Option<u64>,
    pub night: Vec<Offer>,
    pub night_ends_in: Option<u64>,
    pub bundles: Vec<Bundle>,
}

fn offer_for_level(level: &str, c: &Content) -> Offer {
    let level = level.to_lowercase();
    let skin_id = c.levels.get(&level).cloned();
    let skin = skin_id.as_ref().and_then(|s| c.skins.get(s));
    Offer {
        skin_id: skin_id.clone(),
        name: skin.map(|s| s.name.clone()).unwrap_or_else(|| "Bilinmeyen skin".into()),
        image: skin.and_then(|s| s.icon.clone()),
        tier_color: skin
            .and_then(|s| s.tier.as_ref())
            .and_then(|t| c.tiers.get(t))
            .cloned(),
        price: None,
        original: None,
        discount: None,
    }
}

fn first_reward(offer: &Value) -> Option<String> {
    arr(offer, "Rewards").next().and_then(|r| str_at(r, "ItemID"))
}

pub fn store(v: &Value, c: &Content) -> Store {
    let mut st = Store::default();

    let panel = v.get("SkinsPanelLayout").cloned().unwrap_or(Value::Null);
    let offers: Vec<&Value> = arr(&panel, "SingleItemStoreOffers").collect();
    if offers.is_empty() {
        for id in arr(&panel, "SingleItemOffers").filter_map(Value::as_str) {
            st.daily.push(offer_for_level(id, c));
        }
    } else {
        for o in offers {
            if let Some(id) = first_reward(o) {
                let mut item = offer_for_level(&id, c);
                item.price = o.get("Cost").and_then(|x| u64_at(x, VP));
                st.daily.push(item);
            }
        }
    }
    st.daily_ends_in = u64_at(&panel, "SingleItemOffersRemainingDurationInSeconds");

    if let Some(bonus) = v.get("BonusStore") {
        for b in arr(bonus, "BonusStoreOffers") {
            let Some(o) = b.get("Offer") else { continue };
            let Some(id) = first_reward(o) else { continue };
            let mut item = offer_for_level(&id, c);
            item.original = o.get("Cost").and_then(|x| u64_at(x, VP));
            item.price = b.get("DiscountCosts").and_then(|x| u64_at(x, VP));
            item.discount = u64_at(b, "DiscountPercent");
            st.night.push(item);
        }
        st.night_ends_in = u64_at(bonus, "BonusStoreRemainingDurationInSeconds");
    }

    if let Some(fb) = v.get("FeaturedBundle") {
        let mut list: Vec<&Value> = arr(fb, "Bundles").collect();
        if list.is_empty() {
            if let Some(b) = fb.get("Bundle") {
                list.push(b);
            }
        }
        for b in list {
            let Some(asset) = str_at(b, "DataAssetID") else { continue };
            let known = c.bundles.get(&asset.to_lowercase());
            st.bundles.push(Bundle {
                name: known.map(|k| k.0.clone()).unwrap_or_else(|| "Öne çıkan paket".into()),
                image: known.and_then(|k| k.1.clone()),
                ends_in: u64_at(b, "DurationRemainingInSeconds"),
            });
        }
    }
    st
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OwnedSkin {
    pub name: String,
    pub weapon: String,
    pub image: Option<String>,
    pub tier_color: Option<String>,
    #[serde(skip)]
    pub rarity: u64,
}

pub fn collection(v: &Value, c: &Content) -> Vec<OwnedSkin> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for e in arr(v, "Entitlements") {
        let Some(level) = str_at(e, "ItemID") else { continue };
        let Some(skin_id) = c.levels.get(&level.to_lowercase()) else { continue };
        if !seen.insert(skin_id.clone()) {
            continue;
        }
        let Some(skin) = c.skins.get(skin_id) else { continue };
        let weapon = c.weapons.get(&skin.weapon);
        if weapon.and_then(|w| w.default_skin.as_ref()) == Some(skin_id) {
            continue;
        }
        out.push(OwnedSkin {
            name: skin.name.clone(),
            weapon: weapon.map(|w| w.name.clone()).unwrap_or_default(),
            image: skin.icon.clone(),
            tier_color: skin.tier.as_ref().and_then(|t| c.tiers.get(t)).cloned(),
            rarity: skin.tier.as_ref().and_then(|t| c.tier_rank.get(t)).copied().unwrap_or(0),
        });
    }
    out.sort_by(|a, b| b.rarity.cmp(&a.rarity).then(a.weapon.cmp(&b.weapon)).then(a.name.cmp(&b.name)));
    out
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContractView {
    pub name: String,
    pub kind: String,
    pub level: u64,
    pub total: u64,
}

pub fn contracts(v: &Value, act: Option<&str>, c: &Content) -> Vec<ContractView> {
    let progress = |id: &str| {
        arr(v, "Contracts")
            .find(|x| str_at(x, "ContractDefinitionID").map(|d| d.eq_ignore_ascii_case(id)) == Some(true))
            .and_then(|x| u64_at(x, "ProgressionLevelReached"))
            .unwrap_or(0)
    };
    let mut out = Vec::new();

    if let Some(act) = act {
        if let Some((id, def)) = c.contracts.iter().find(|(_, d)| d.season.as_deref() == Some(act)) {
            out.push(ContractView {
                name: def.name.clone(),
                kind: "Battle Pass".into(),
                level: progress(id.as_str()),
                total: def.levels,
            });
        }
    }
    if let Some(id) = str_at(v, "ActiveSpecialContract") {
        if let Some(def) = c.contracts.get(&id.to_lowercase()) {
            out.push(ContractView {
                name: def.name.clone(),
                kind: "Aktif kontrat".into(),
                level: progress(id.as_str()),
                total: def.levels,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RrChange {
    pub map: Option<String>,
    pub tier: TierBadge,
    pub rr_after: u64,
    pub earned: i64,
    pub started: u64,
}

pub fn rr_history(v: &Value, c: &Content) -> Vec<RrChange> {
    arr(v, "Matches")
        .map(|m| RrChange {
            map: str_at(m, "MapID").and_then(|p| c.map_name(&p)),
            tier: c.rank(u64_at(m, "TierAfterUpdate").unwrap_or(0)),
            rr_after: u64_at(m, "RankedRatingAfterUpdate").unwrap_or(0),
            earned: m.get("RankedRatingEarned").and_then(Value::as_i64).unwrap_or(0),
            started: u64_at(m, "MatchStartTime").unwrap_or(0),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MissionView {
    pub title: String,
    /// daily | weekly | other
    pub kind: String,
    pub xp: u64,
    pub progress: u64,
    pub target: u64,
    pub complete: bool,
    /// ISO time from Riot, e.g. 2026-10-20T00:00:00Z
    pub expires: Option<String>,
}

pub fn missions(v: &Value, c: &Content) -> Vec<MissionView> {
    let mut out: Vec<MissionView> = arr(v, "Missions")
        .filter_map(|m| {
            let id = str_at(m, "ID")?.to_lowercase();
            let def = c.missions.get(&id);
            let progress = m
                .get("Objectives")
                .and_then(Value::as_object)
                .map(|o| o.values().filter_map(Value::as_u64).max().unwrap_or(0))
                .unwrap_or(0);
            let target = def.map(|d| d.target).filter(|t| *t > 0).unwrap_or(progress.max(1));
            Some(MissionView {
                title: def.map(|d| d.title.clone()).unwrap_or_else(|| "Görev".into()),
                kind: def.map(|d| d.kind.clone()).unwrap_or_else(|| "other".into()),
                xp: def.map(|d| d.xp).unwrap_or(0),
                progress: progress.min(target),
                target,
                complete: m.get("Complete").and_then(Value::as_bool).unwrap_or(false),
                expires: str_at(m, "ExpirationTime"),
            })
        })
        .collect();
    let order = |k: &str| match k {
        "daily" => 0,
        "weekly" => 1,
        _ => 2,
    };
    out.sort_by_key(|m| (m.complete, order(&m.kind)));
    out
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct AccountView {
    pub puuid: String,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub level: Option<u64>,
    pub card_wide: Option<String>,
    pub card_small: Option<String>,
    pub title: Option<String>,
    pub rank: Option<RankInfo>,
    pub rr_history: Vec<RrChange>,
    pub wallet: Option<Wallet>,
    pub store: Option<Store>,
    pub collection: Vec<OwnedSkin>,
    pub contracts: Vec<ContractView>,
    pub missions: Vec<MissionView>,
    pub weekly_refill: Option<String>,
    pub border: Option<String>,
    pub problems: Vec<String>,
}
