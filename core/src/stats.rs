//! Per-player numbers from Riot match details.

use crate::content::{Content, TierBadge};
use crate::riot::{str_at, u64_at};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MatchLine {
    pub match_id: String,
    pub map: Option<String>,
    pub queue: String,
    pub agent: Option<String>,
    pub agent_icon: Option<String>,
    /// None for modes without a winning team (deathmatch).
    pub won: Option<bool>,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub acs: u64,
    pub hs: Option<u32>,
    pub rounds_us: u64,
    pub rounds_them: u64,
    pub started: u64,
    pub tier: Option<TierBadge>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AgentCount {
    pub name: String,
    pub icon: Option<String>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct PlayerStats {
    pub matches: Vec<MatchLine>,
    pub kd: Option<f64>,
    pub acs: Option<u64>,
    pub hs: Option<u32>,
    pub winrate: Option<u32>,
    pub agents: Vec<AgentCount>,
}

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

pub fn queue_label(q: &str) -> String {
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

/// Head-shot percentage for one player across all rounds.
fn headshots(details: &Value, puuid: &str) -> Option<u32> {
    let (mut head, mut total) = (0u64, 0u64);
    for round in arr(details, "roundResults") {
        for ps in arr(round, "playerStats") {
            if str_at(ps, "subject").as_deref() != Some(puuid) {
                continue;
            }
            for d in arr(ps, "damage") {
                let h = u64_at(d, "headshots").unwrap_or(0);
                head += h;
                total += h + u64_at(d, "bodyshots").unwrap_or(0) + u64_at(d, "legshots").unwrap_or(0);
            }
        }
    }
    if total == 0 {
        None
    } else {
        Some(((head * 100) as f64 / total as f64).round() as u32)
    }
}

pub fn line(details: &Value, puuid: &str, c: &Content) -> Option<MatchLine> {
    let info = details.get("matchInfo")?;
    let me = arr(details, "players").find(|p| str_at(p, "subject").as_deref() == Some(puuid))?;
    let team = str_at(me, "teamId").unwrap_or_default();
    let st = me.get("stats")?;
    let rounds = u64_at(st, "roundsPlayed").unwrap_or(0);
    let score = u64_at(st, "score").unwrap_or(0);

    let teams: Vec<&Value> = arr(details, "teams").collect();
    let mine = teams.iter().find(|t| str_at(t, "teamId").as_deref() == Some(team.as_str()));
    let theirs = teams.iter().find(|t| str_at(t, "teamId").as_deref() != Some(team.as_str()));
    let won = if teams.len() == 2 {
        mine.and_then(|t| t.get("won")).and_then(Value::as_bool)
    } else {
        None
    };

    let agent = str_at(me, "characterId").and_then(|a| c.agents.get(&a.to_lowercase()).cloned());

    Some(MatchLine {
        match_id: str_at(info, "matchId").unwrap_or_default(),
        map: str_at(info, "mapId").and_then(|m| c.map_name(&m)),
        queue: queue_label(&str_at(info, "queueID").unwrap_or_default()),
        agent: agent.as_ref().map(|a| a.name.clone()),
        agent_icon: agent.and_then(|a| a.icon),
        won,
        kills: u64_at(st, "kills").unwrap_or(0),
        deaths: u64_at(st, "deaths").unwrap_or(0),
        assists: u64_at(st, "assists").unwrap_or(0),
        acs: if rounds > 0 { score / rounds } else { 0 },
        hs: headshots(details, puuid),
        rounds_us: mine.and_then(|t| u64_at(t, "roundsWon")).unwrap_or(0),
        rounds_them: theirs.and_then(|t| u64_at(t, "roundsWon")).unwrap_or(0),
        started: u64_at(info, "gameStartMillis").unwrap_or(0),
        tier: u64_at(me, "competitiveTier").filter(|t| *t > 0).map(|t| c.rank(t)),
    })
}

pub fn summarize(matches: Vec<MatchLine>) -> PlayerStats {
    if matches.is_empty() {
        return PlayerStats::default();
    }
    let kills: u64 = matches.iter().map(|m| m.kills).sum();
    let deaths: u64 = matches.iter().map(|m| m.deaths).sum();
    let acs = matches.iter().map(|m| m.acs).sum::<u64>() / matches.len() as u64;
    let hs: Vec<u32> = matches.iter().filter_map(|m| m.hs).collect();
    let decided: Vec<bool> = matches.iter().filter_map(|m| m.won).collect();

    let mut agents: Vec<AgentCount> = Vec::new();
    for m in &matches {
        let Some(name) = &m.agent else { continue };
        let pos = agents.iter().position(|a| &a.name == name);
        match pos {
            Some(i) => agents[i].count += 1,
            None => agents.push(AgentCount {
                name: name.clone(),
                icon: m.agent_icon.clone(),
                count: 1,
            }),
        }
    }
    agents.sort_by(|a, b| b.count.cmp(&a.count));
    agents.truncate(3);

    PlayerStats {
        kd: Some(((kills as f64 / deaths.max(1) as f64) * 100.0).round() / 100.0),
        acs: Some(acs),
        hs: if hs.is_empty() {
            None
        } else {
            Some(hs.iter().sum::<u32>() / hs.len() as u32)
        },
        winrate: if decided.is_empty() {
            None
        } else {
            Some((decided.iter().filter(|w| **w).count() * 100 / decided.len()) as u32)
        },
        agents,
        matches,
    }
}

/// Pairs of current players who queued together in a recent match.
pub fn parties_in(details: &Value, current: &[String]) -> Vec<Vec<String>> {
    let mut by_party: Vec<(String, Vec<String>)> = Vec::new();
    for p in arr(details, "players") {
        let (Some(subject), Some(party)) = (str_at(p, "subject"), str_at(p, "partyId")) else {
            continue;
        };
        if !current.contains(&subject) {
            continue;
        }
        let pos = by_party.iter().position(|(id, _)| *id == party);
        match pos {
            Some(i) => by_party[i].1.push(subject),
            None => by_party.push((party, vec![subject])),
        }
    }
    by_party
        .into_iter()
        .map(|(_, m)| m)
        .filter(|m| m.len() > 1)
        .collect()
}
