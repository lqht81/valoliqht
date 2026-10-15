//! Your own recent matches: who you met before, and full scoreboards.

use crate::content::{Content, TierBadge};
use crate::riot::{str_at, u64_at};
use crate::stats::queue_label;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

fn arr<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v.get(key).and_then(Value::as_array).into_iter().flatten()
}

/// One player's numbers in one match.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Line {
    pub puuid: String,
    pub team: String,
    pub party: Option<String>,
    pub agent_id: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub score: u64,
    pub rounds: u64,
    pub damage: u64,
    pub head: u64,
    pub shots: u64,
    pub tier: u64,
}

impl Line {
    pub fn acs(&self) -> u64 {
        if self.rounds > 0 {
            self.score / self.rounds
        } else {
            0
        }
    }
    pub fn adr(&self) -> u64 {
        if self.rounds > 0 {
            self.damage / self.rounds
        } else {
            0
        }
    }
    pub fn hs(&self) -> Option<u32> {
        if self.shots == 0 {
            None
        } else {
            Some(((self.head * 100) as f64 / self.shots as f64).round() as u32)
        }
    }
}

pub fn lines(details: &Value) -> Vec<Line> {
    // Damage and shot counts live per round, per attacker.
    let mut dmg: HashMap<String, (u64, u64, u64)> = HashMap::new();
    for round in arr(details, "roundResults") {
        for ps in arr(round, "playerStats") {
            let Some(subject) = str_at(ps, "subject") else { continue };
            let entry = dmg.entry(subject).or_default();
            for d in arr(ps, "damage") {
                let h = u64_at(d, "headshots").unwrap_or(0);
                entry.0 += u64_at(d, "damage").unwrap_or(0);
                entry.1 += h;
                entry.2 += h + u64_at(d, "bodyshots").unwrap_or(0) + u64_at(d, "legshots").unwrap_or(0);
            }
        }
    }
    arr(details, "players")
        .filter_map(|p| {
            let puuid = str_at(p, "subject")?;
            let st = p.get("stats").cloned().unwrap_or(Value::Null);
            let (damage, head, shots) = dmg.get(&puuid).copied().unwrap_or_default();
            Some(Line {
                team: str_at(p, "teamId").unwrap_or_default(),
                party: str_at(p, "partyId"),
                agent_id: str_at(p, "characterId").map(|x| x.to_lowercase()),
                name: str_at(p, "gameName"),
                tag: str_at(p, "tagLine"),
                kills: u64_at(&st, "kills").unwrap_or(0),
                deaths: u64_at(&st, "deaths").unwrap_or(0),
                assists: u64_at(&st, "assists").unwrap_or(0),
                score: u64_at(&st, "score").unwrap_or(0),
                rounds: u64_at(&st, "roundsPlayed").unwrap_or(0),
                damage,
                head,
                shots,
                tier: u64_at(p, "competitiveTier").unwrap_or(0),
                puuid,
            })
        })
        .collect()
}

fn team_result(details: &Value, team: &str) -> (Option<bool>, u64, u64) {
    let teams: Vec<&Value> = arr(details, "teams").collect();
    let mine = teams.iter().find(|t| str_at(t, "teamId").as_deref() == Some(team));
    let theirs = teams.iter().find(|t| str_at(t, "teamId").as_deref() != Some(team));
    let won = if teams.len() == 2 {
        mine.and_then(|t| t.get("won")).and_then(Value::as_bool)
    } else {
        None
    };
    (
        won,
        mine.and_then(|t| u64_at(t, "roundsWon")).unwrap_or(0),
        theirs.and_then(|t| u64_at(t, "roundsWon")).unwrap_or(0),
    )
}

/// A compact copy of one of your matches, kept in memory while the app runs.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnMatch {
    pub id: String,
    pub map: Option<String>,
    pub queue: String,
    pub started: u64,
    pub my_team: String,
    pub my_won: Option<bool>,
    pub players: Vec<Line>,
}

pub fn own_match(details: &Value, me: &str, c: &Content) -> Option<OwnMatch> {
    let info = details.get("matchInfo")?;
    let players = lines(details);
    let my_team = players.iter().find(|p| p.puuid == me)?.team.clone();
    let (my_won, _, _) = team_result(details, &my_team);
    Some(OwnMatch {
        id: str_at(info, "matchId").unwrap_or_default(),
        map: str_at(info, "mapId").and_then(|m| c.map_name(&m)),
        queue: queue_label(&str_at(info, "queueID").unwrap_or_default()),
        started: u64_at(info, "gameStartMillis").unwrap_or(0),
        my_team,
        my_won,
        players,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Encounter {
    /// How many of your recent matches they were in.
    pub times: u32,
    /// The latest one: 1 = your previous match.
    pub ago: u32,
    pub ally: bool,
    pub map: Option<String>,
    /// Whether *you* won that match.
    pub won: Option<bool>,
    pub agent: Option<String>,
    pub agent_icon: Option<String>,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub acs: u64,
}

/// `history` is newest first.
pub fn encounter(history: &[OwnMatch], puuid: &str, c: &Content) -> Option<Encounter> {
    let mut found: Option<(usize, &OwnMatch, &Line)> = None;
    let mut times = 0u32;
    for (i, m) in history.iter().enumerate() {
        if let Some(line) = m.players.iter().find(|p| p.puuid == puuid) {
            times += 1;
            if found.is_none() {
                found = Some((i, m, line));
            }
        }
    }
    let (i, m, line) = found?;
    let agent = line.agent_id.as_ref().and_then(|a| c.agents.get(a));
    Some(Encounter {
        times,
        ago: i as u32 + 1,
        ally: line.team == m.my_team,
        map: m.map.clone(),
        won: m.my_won,
        agent: agent.map(|a| a.name.clone()),
        agent_icon: agent.and_then(|a| a.icon.clone()),
        kills: line.kills,
        deaths: line.deaths,
        assists: line.assists,
        acs: line.acs(),
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScoreRow {
    pub puuid: String,
    /// Only for you, your party in that match and your friends.
    pub name: Option<String>,
    pub tag: Option<String>,
    pub agent: Option<String>,
    pub agent_icon: Option<String>,
    pub tier: Option<TierBadge>,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub acs: u64,
    pub adr: u64,
    pub hs: Option<u32>,
    pub is_me: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Scoreboard {
    pub match_id: String,
    pub map: Option<String>,
    pub queue: String,
    pub started: u64,
    pub minutes: u64,
    pub won: Option<bool>,
    pub rounds_us: u64,
    pub rounds_them: u64,
    pub mine: Vec<ScoreRow>,
    pub theirs: Vec<ScoreRow>,
}

pub fn scoreboard(details: &Value, me: &str, c: &Content, friends: &HashSet<String>) -> Option<Scoreboard> {
    let info = details.get("matchInfo")?;
    let players = lines(details);
    let mine_line = players.iter().find(|p| p.puuid == me)?;
    let my_team = mine_line.team.clone();
    let my_party = mine_line.party.clone();
    let (won, us, them) = team_result(details, &my_team);

    let row = |p: &Line| {
        let known = p.puuid == me
            || friends.contains(&p.puuid)
            || (my_party.is_some() && p.party == my_party);
        let agent = p.agent_id.as_ref().and_then(|a| c.agents.get(a));
        ScoreRow {
            puuid: p.puuid.clone(),
            name: if known { p.name.clone() } else { None },
            tag: if known { p.tag.clone() } else { None },
            agent: agent.map(|a| a.name.clone()),
            agent_icon: agent.and_then(|a| a.icon.clone()),
            tier: if p.tier > 0 { Some(c.rank(p.tier)) } else { None },
            kills: p.kills,
            deaths: p.deaths,
            assists: p.assists,
            acs: p.acs(),
            adr: p.adr(),
            hs: p.hs(),
            is_me: p.puuid == me,
        }
    };
    let mut mine: Vec<ScoreRow> = players.iter().filter(|p| p.team == my_team).map(row).collect();
    let mut theirs: Vec<ScoreRow> = players.iter().filter(|p| p.team != my_team).map(row).collect();
    mine.sort_by(|a, b| b.acs.cmp(&a.acs));
    theirs.sort_by(|a, b| b.acs.cmp(&a.acs));

    Some(Scoreboard {
        match_id: str_at(info, "matchId").unwrap_or_default(),
        map: str_at(info, "mapId").and_then(|m| c.map_name(&m)),
        queue: queue_label(&str_at(info, "queueID").unwrap_or_default()),
        started: u64_at(info, "gameStartMillis").unwrap_or(0),
        minutes: u64_at(info, "gameLengthMillis").unwrap_or(0) / 60000,
        won,
        rounds_us: us,
        rounds_them: them,
        mine,
        theirs,
    })
}
