//! Reads the current VALORANT match, your account and your friends.
//!
//! Hosts contacted: 127.0.0.1 (Riot Client), Riot's servers (*.a.pvp.net)
//! and valorant-api.com (public skin database). Nothing else.

pub mod account;
pub mod content;
pub mod local;
pub mod ranks;
pub mod riot;
pub mod social;
pub mod stats;
pub mod view;

pub use account::AccountView;
pub use social::FriendView;
pub use stats::PlayerStats;
pub use view::Snapshot;

use content::Content;
use ranks::RankInfo;
use riot::{str_at, Riot, Session};
use serde_json::Value;
use social::Presence;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use view::RawPlayer;

const RANK_TTL: Duration = Duration::from_secs(600);
const STATS_TTL: Duration = Duration::from_secs(300);
const ACCOUNT_TTL: Duration = Duration::from_secs(60);
const STATS_MATCHES: usize = 8;

struct Timed<T> {
    at: Instant,
    value: T,
}

impl<T> Timed<T> {
    fn now(value: T) -> Self {
        Timed { at: Instant::now(), value }
    }
}

fn fresh<T: Clone>(m: &Mutex<HashMap<String, Timed<T>>>, key: &str, ttl: Duration) -> Option<T> {
    let guard = m.lock().unwrap();
    guard
        .get(key)
        .filter(|t| t.at.elapsed() < ttl)
        .map(|t| t.value.clone())
}

struct Inner {
    riot: Riot,
    content: tokio::sync::Mutex<Option<Arc<Content>>>,
    /// (shard, active act id)
    act: Mutex<Option<(String, Option<String>)>>,
    names: Mutex<HashMap<String, (String, String)>>,
    ranks: Mutex<HashMap<String, Timed<RankInfo>>>,
    stats: Mutex<HashMap<String, Timed<PlayerStats>>>,
    details: Mutex<HashMap<String, Arc<Value>>>,
    /// match id -> groups of players who queued together recently
    parties: Mutex<HashMap<String, Vec<Vec<String>>>>,
    /// Enrichment jobs running / finished, keyed by match id (+ mode).
    busy: Mutex<HashSet<String>>,
    done: Mutex<HashSet<String>>,
    /// Premade groups, peak rank and other players' history. Off by default.
    advanced: AtomicBool,
    account: Mutex<Option<Timed<AccountView>>>,
}

impl Inner {
    async fn content(&self) -> Result<Arc<Content>, String> {
        let mut slot = self.content.lock().await;
        if let Some(c) = slot.as_ref() {
            return Ok(c.clone());
        }
        let c = Arc::new(Content::download(self.riot.remote()).await?);
        *slot = Some(c.clone());
        Ok(c)
    }

    /// Lockfile → session → content, as a (status, message) on failure.
    async fn ready(&self) -> Result<(Session, Arc<Content>), (&'static str, String)> {
        let Some(lf) = local::read_lockfile() else {
            return Err(("riot_closed", "Riot Client açık değil.".into()));
        };
        let log = local::read_log();
        let s = self.riot.session(&lf, &log).await.map_err(|e| ("waiting", e))?;
        let c = self
            .content()
            .await
            .map_err(|e| ("error", format!("Skin verileri indirilemedi: {e}")))?;
        Ok((s, c))
    }

    async fn act(&self, s: &Session) -> Option<String> {
        let cached = self.act.lock().unwrap().clone();
        if let Some((shard, act)) = cached {
            if shard == s.shard {
                return act;
            }
        }
        let act = match self.riot.shared(s, "/content-service/v3/content").await {
            Ok(Some(v)) => ranks::active_act(&v),
            _ => None,
        };
        if act.is_some() {
            *self.act.lock().unwrap() = Some((s.shard.clone(), act.clone()));
        }
        act
    }

    async fn presences(&self, s: &Session) -> Vec<Presence> {
        match self.riot.local_get(&s.lockfile, "/chat/v4/presences").await {
            Ok(v) => social::parse_presences(&v),
            Err(_) => Vec::new(),
        }
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
        let cache = self.names.lock().unwrap();
        cache.clone()
    }

    async fn history(&self, s: &Session, puuid: &str, count: usize) -> Result<Vec<String>, String> {
        let path = format!("/match-history/v1/history/{puuid}?startIndex=0&endIndex={count}");
        let v = self.riot.pd(s, &path).await?.unwrap_or(Value::Null);
        Ok(v.get("History")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|h| str_at(h, "MatchID"))
            .collect())
    }

    async fn details(&self, s: &Session, id: &str) -> Option<Arc<Value>> {
        let hit = self.details.lock().unwrap().get(id).cloned();
        if hit.is_some() {
            return hit;
        }
        let path = format!("/match-details/v1/matches/{id}");
        let v = Arc::new(self.riot.pd(s, &path).await.ok().flatten()?);
        {
            let mut cache = self.details.lock().unwrap();
            if cache.len() > 60 {
                cache.clear();
            }
            cache.insert(id.to_string(), v.clone());
        }
        Some(v)
    }

    async fn rank_of(&self, s: &Session, c: &Content, act: Option<&str>, puuid: &str) -> Result<RankInfo, String> {
        let v = self
            .riot
            .pd(s, &format!("/mmr/v1/players/{puuid}"))
            .await?
            .unwrap_or(Value::Null);
        let r = ranks::from_mmr(&v, act, c);
        self.ranks.lock().unwrap().insert(puuid.to_string(), Timed::now(r.clone()));
        Ok(r)
    }

    /// Background work for a match: everyone's rank, then who queued with
    /// whom in their previous game.
    async fn enrich(
        self: Arc<Self>,
        s: Session,
        c: Arc<Content>,
        match_id: String,
        players: Vec<String>,
        advanced: bool,
        key: String,
    ) {
        let act = self.act(&s).await;
        for p in &players {
            if fresh(&self.ranks, p, RANK_TTL).is_none() {
                let _ = self.rank_of(&s, &c, act.as_deref(), p).await;
            }
        }
        if !advanced {
            self.finish(&key);
            return;
        }

        let mut previous: Vec<String> = Vec::new();
        for p in &players {
            if let Ok(ids) = self.history(&s, p, 3).await {
                if let Some(id) = ids.into_iter().find(|id| *id != match_id) {
                    if !previous.contains(&id) {
                        previous.push(id);
                    }
                }
            }
        }
        let mut groups = Vec::new();
        for id in previous {
            if let Some(d) = self.details(&s, &id).await {
                groups.extend(stats::parties_in(&d, &players));
            }
        }

        {
            let mut parties = self.parties.lock().unwrap();
            if parties.len() > 20 {
                parties.clear();
            }
            parties.insert(match_id.clone(), groups);
        }
        self.finish(&key);
    }

    fn finish(&self, key: &str) {
        {
            let mut done = self.done.lock().unwrap();
            if done.len() > 50 {
                done.clear();
            }
            done.insert(key.to_string());
        }
        self.busy.lock().unwrap().remove(key);
    }

    fn advanced(&self) -> bool {
        self.advanced.load(Ordering::Relaxed)
    }
}

pub struct Engine {
    inner: Arc<Inner>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                riot: Riot::new(),
                content: tokio::sync::Mutex::new(None),
                act: Mutex::new(None),
                names: Mutex::new(HashMap::new()),
                ranks: Mutex::new(HashMap::new()),
                stats: Mutex::new(HashMap::new()),
                details: Mutex::new(HashMap::new()),
                parties: Mutex::new(HashMap::new()),
                busy: Mutex::new(HashSet::new()),
                done: Mutex::new(HashSet::new()),
                advanced: AtomicBool::new(false),
                account: Mutex::new(None),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn build(
        &self,
        s: &Session,
        c: &Arc<Content>,
        phase: &str,
        match_id: String,
        m: &Value,
        raw: Vec<RawPlayer>,
        loadouts: &Value,
        presences: &[Presence],
    ) -> Snapshot {
        let i = &self.inner;
        let by_player = view::loadouts_by_player(loadouts, &raw);
        let names = i.resolve_names(s, &raw).await;
        let roster: Vec<String> = raw.iter().map(|p| p.puuid.clone()).collect();

        let advanced = i.advanced();
        let key = if advanced { format!("{match_id}+") } else { match_id.clone() };
        let start = {
            let known = i.done.lock().unwrap().contains(&key);
            !known && i.busy.lock().unwrap().insert(key.clone())
        };
        if start {
            tokio::spawn(Arc::clone(i).enrich(
                s.clone(),
                c.clone(),
                match_id.clone(),
                roster.clone(),
                advanced,
                key,
            ));
        }

        let ranks: HashMap<String, RankInfo> = {
            let cache = i.ranks.lock().unwrap();
            roster
                .iter()
                .filter_map(|p| {
                    cache.get(p).map(|t| {
                        let mut r = t.value.clone();
                        if !advanced {
                            r.peak = None;
                        }
                        (p.clone(), r)
                    })
                })
                .collect()
        };
        let parties = if advanced {
            let mut groups = i
                .parties
                .lock()
                .unwrap()
                .get(&match_id)
                .cloned()
                .unwrap_or_default();
            groups.extend(social::party_groups(presences));
            view::number_parties(&groups, &roster)
        } else {
            HashMap::new()
        };

        let extras = view::Extras {
            names: &names,
            ranks: &ranks,
            parties: &parties,
        };
        let players = view::assemble(&raw, &by_player, c, &s.puuid, &extras);
        let queue = str_at(m, "QueueID")
            .or_else(|| m.pointer("/MatchmakingData/QueueID").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        let score = if phase == "ingame" {
            presences.iter().find(|p| p.puuid == s.puuid).and_then(|p| p.score())
        } else {
            None
        };
        Snapshot {
            status: phase.into(),
            message: None,
            match_id: Some(match_id),
            map: str_at(m, "MapID").and_then(|id| c.map_name(&id)),
            queue: Some(stats::queue_label(&queue)),
            score,
            advanced,
            weapons: view::weapon_meta(&players, c),
            players,
        }
    }

    pub async fn snapshot(&self) -> Snapshot {
        let i = &self.inner;
        let (s, c) = match i.ready().await {
            Ok(x) => x,
            Err((status, msg)) => return Snapshot::status(status, msg),
        };
        let presences = i.presences(&s).await;

        // In a match (or loading into one).
        match i.riot.glz(&s, &format!("/core-game/v1/players/{}", s.puuid)).await {
            Ok(Some(v)) => {
                if let Some(id) = str_at(&v, "MatchID") {
                    let m = i.riot.glz(&s, &format!("/core-game/v1/matches/{id}")).await;
                    let l = i.riot.glz(&s, &format!("/core-game/v1/matches/{id}/loadouts")).await;
                    if let (Ok(Some(m)), Ok(Some(l))) = (m, l) {
                        let raw = view::players_from_core(&m);
                        return self.build(&s, &c, "ingame", id, &m, raw, &l, &presences).await;
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Snapshot::status("error", e),
        }

        // Agent select.
        match i.riot.glz(&s, &format!("/pregame/v1/players/{}", s.puuid)).await {
            Ok(Some(v)) => {
                if let Some(id) = str_at(&v, "MatchID") {
                    let m = i.riot.glz(&s, &format!("/pregame/v1/matches/{id}")).await;
                    let l = i.riot.glz(&s, &format!("/pregame/v1/matches/{id}/loadouts")).await;
                    if let Ok(Some(m)) = m {
                        let raw = view::players_from_pregame(&m);
                        let l = l.ok().flatten().unwrap_or(Value::Null);
                        return self.build(&s, &c, "pregame", id, &m, raw, &l, &presences).await;
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Snapshot::status("error", e),
        }

        Snapshot::status(
            "menu",
            "Ajan seçimine girdiğinde takımının, maç başlayınca herkesin bilgileri görünür.",
        )
    }

    /// Recent matches of one player, loaded when their card is opened.
    pub fn set_advanced(&self, on: bool) {
        self.inner.advanced.store(on, Ordering::Relaxed);
    }

    pub async fn player_stats(&self, puuid: String) -> Result<PlayerStats, String> {
        let i = &self.inner;
        if !i.advanced() {
            let me = local::read_lockfile().is_some()
                && i.ready().await.map(|(s, _)| s.puuid == puuid).unwrap_or(false);
            if !me {
                return Err("Diğer oyuncuların maç geçmişi için Gelişmiş bilgileri aç.".into());
            }
        }
        if let Some(v) = fresh(&i.stats, &puuid, STATS_TTL) {
            return Ok(v);
        }
        let (s, c) = i.ready().await.map_err(|e| e.1)?;
        let ids = i.history(&s, &puuid, STATS_MATCHES).await?;
        let mut lines = Vec::new();
        for id in ids {
            if let Some(d) = i.details(&s, &id).await {
                if let Some(l) = stats::line(&d, &puuid, &c) {
                    lines.push(l);
                }
            }
        }
        let st = stats::summarize(lines);
        i.stats.lock().unwrap().insert(puuid, Timed::now(st.clone()));
        Ok(st)
    }

    pub async fn friends(&self) -> Result<Vec<FriendView>, String> {
        let i = &self.inner;
        let (s, c) = i.ready().await.map_err(|e| e.1)?;
        let list = i.riot.local_get(&s.lockfile, "/chat/v4/friends").await?;
        let presences = i.presences(&s).await;
        Ok(social::friends(&list, &presences, &c))
    }

    pub async fn account(&self, force: bool) -> Result<AccountView, String> {
        let i = &self.inner;
        if !force {
            let cached = {
                let slot = i.account.lock().unwrap();
                slot.as_ref()
                    .filter(|t| t.at.elapsed() < ACCOUNT_TTL)
                    .map(|t| t.value.clone())
            };
            if let Some(v) = cached {
                return Ok(v);
            }
        }
        let (s, c) = i.ready().await.map_err(|e| e.1)?;
        let act = i.act(&s).await;
        let me = s.puuid.clone();
        let mut view = AccountView {
            puuid: me.clone(),
            ..Default::default()
        };
        let mut problems: Vec<String> = Vec::new();
        let mut note = |what: &str, e: String| problems.push(format!("{what}: {e}"));

        if let Ok(found) = i.riot.names(&s, &[me.clone()]).await {
            if let Some((_, name, tag)) = found.into_iter().next() {
                view.name = Some(name);
                view.tag = Some(tag);
            }
        }

        match i.riot.pd(&s, &format!("/account-xp/v1/players/{me}")).await {
            Ok(Some(v)) => view.level = v.pointer("/Progress/Level").and_then(Value::as_u64),
            Ok(None) => {}
            Err(e) => note("Seviye", e),
        }

        match i.riot.pd(&s, &format!("/personalization/v2/players/{me}/playerloadout")).await {
            Ok(Some(v)) => {
                let card = v
                    .pointer("/Identity/PlayerCardID")
                    .and_then(Value::as_str)
                    .and_then(|id| c.cards.get(&id.to_lowercase()));
                view.card_wide = card.and_then(|k| k.wide.clone());
                view.card_small = card.and_then(|k| k.small.clone());
                view.title = v
                    .pointer("/Identity/PlayerTitleID")
                    .and_then(Value::as_str)
                    .and_then(|id| c.titles.get(&id.to_lowercase()))
                    .cloned();
            }
            Ok(None) => {}
            Err(e) => note("Kart", e),
        }

        match i.rank_of(&s, &c, act.as_deref(), &me).await {
            Ok(r) => view.rank = Some(r),
            Err(e) => note("Rank", e),
        }

        let rr_path = format!("/mmr/v1/players/{me}/competitiveupdates?startIndex=0&endIndex=10&queue=competitive");
        match i.riot.pd(&s, &rr_path).await {
            Ok(Some(v)) => view.rr_history = account::rr_history(&v, &c),
            Ok(None) => {}
            Err(e) => note("RR geçmişi", e),
        }

        match i.riot.pd(&s, &format!("/store/v1/wallet/{me}")).await {
            Ok(Some(v)) => view.wallet = Some(account::wallet(&v)),
            Ok(None) => {}
            Err(e) => note("Cüzdan", e),
        }

        let empty = Value::Object(Default::default());
        let store = match i.riot.pd_post(&s, &format!("/store/v3/storefront/{me}"), &empty).await {
            Ok(Some(v)) => Ok(Some(v)),
            _ => i.riot.pd(&s, &format!("/store/v2/storefront/{me}")).await,
        };
        match store {
            Ok(Some(v)) => view.store = Some(account::store(&v, &c)),
            Ok(None) => {}
            Err(e) => note("Mağaza", e),
        }

        let ent_path = format!("/store/v1/entitlements/{me}/{}", account::SKIN_LEVEL_TYPE);
        match i.riot.pd(&s, &ent_path).await {
            Ok(Some(v)) => view.collection = account::collection(&v, &c),
            Ok(None) => {}
            Err(e) => note("Koleksiyon", e),
        }

        match i.riot.pd(&s, &format!("/contracts/v1/contracts/{me}")).await {
            Ok(Some(v)) => view.contracts = account::contracts(&v, act.as_deref(), &c),
            Ok(None) => {}
            Err(e) => note("Kontratlar", e),
        }

        view.problems = problems;
        *i.account.lock().unwrap() = Some(Timed::now(view.clone()));
        Ok(view)
    }
}

#[cfg(test)]
mod tests;
