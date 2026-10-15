//! Current and peak competitive rank from Riot's MMR data.

use crate::content::{Content, TierBadge};
use crate::riot::{str_at, u64_at};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct RankInfo {
    pub current: TierBadge,
    pub rr: Option<u64>,
    pub peak: Option<TierBadge>,
    pub wins: Option<u64>,
    pub games: Option<u64>,
    pub leaderboard: Option<u64>,
}

/// The act that is live right now, from the shared content service.
pub fn active_act(content: &Value) -> Option<String> {
    content
        .get("Seasons")
        .and_then(Value::as_array)?
        .iter()
        .find(|s| {
            s.get("IsActive").and_then(Value::as_bool) == Some(true)
                && s.get("Type").and_then(Value::as_str).map(|t| t.eq_ignore_ascii_case("act")) == Some(true)
        })
        .and_then(|s| str_at(s, "ID"))
        .map(|x| x.to_lowercase())
}

pub fn from_mmr(v: &Value, act: Option<&str>, c: &Content) -> RankInfo {
    let seasons = v
        .pointer("/QueueSkills/competitive/SeasonalInfoBySeasonID")
        .and_then(Value::as_object);

    let mut tier = 0;
    let mut rr = None;
    let mut wins = None;
    let mut games = None;
    let mut leaderboard = None;

    if let (Some(map), Some(act)) = (seasons, act) {
        if let Some(info) = map
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(act))
            .map(|(_, v)| v)
        {
            tier = u64_at(info, "CompetitiveTier").unwrap_or(0);
            rr = u64_at(info, "RankedRating");
            wins = u64_at(info, "NumberOfWinsWithPlacements").or_else(|| u64_at(info, "NumberOfWins"));
            games = u64_at(info, "NumberOfGames");
            leaderboard = u64_at(info, "LeaderboardRank").filter(|x| *x > 0);
        }
    }

    if tier == 0 {
        if let (Some(lu), Some(act)) = (v.get("LatestCompetitiveUpdate"), act) {
            let same_act = str_at(lu, "SeasonID")
                .map(|s| s.eq_ignore_ascii_case(act))
                .unwrap_or(false);
            if same_act {
                tier = u64_at(lu, "TierAfterUpdate").unwrap_or(0);
                rr = u64_at(lu, "RankedRatingAfterUpdate");
            }
        }
    }

    let mut peak = tier;
    for info in seasons.into_iter().flat_map(|m| m.values()) {
        if let Some(by_tier) = info.get("WinsByTier").and_then(Value::as_object) {
            for k in by_tier.keys() {
                if let Ok(t) = k.parse::<u64>() {
                    peak = peak.max(t);
                }
            }
        }
    }

    RankInfo {
        current: c.rank(tier),
        rr: if tier > 0 { rr } else { None },
        peak: if peak > 0 { Some(c.rank(peak)) } else { None },
        wins,
        games,
        leaderboard,
    }
}
