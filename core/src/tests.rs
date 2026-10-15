use crate::account::{self, RP, VP};
use crate::content::{Content, Sources};
use crate::ranks;
use crate::social;
use crate::stats;
use crate::view::*;
use serde_json::{json, Value};
use std::collections::HashMap;

const VANDAL: &str = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const CLASSIC: &str = "29a0cfab-485b-f5d5-779a-b59f85e204a8";
const ACT: &str = "act-now";

fn content() -> Content {
    let src = Sources {
        agents: json!({ "data": [
            { "uuid": "AAAA-JETT", "displayName": "Jett", "displayIconSmall": "https://img/jett.png" },
            { "uuid": "aaaa-sage", "displayName": "Sage", "displayIcon": "https://img/sage.png" }
        ]}),
        weapons: json!({ "data": [
            {
                "uuid": VANDAL, "displayName": "Vandal", "category": "EEquippableCategory::Rifle",
                "displayIcon": "https://img/vandal.png", "defaultSkinUuid": "skin-vandal-std",
                "skins": [
                    { "uuid": "skin-vandal-std", "displayName": "Standart Vandal", "contentTierUuid": null,
                      "displayIcon": null, "chromas": [],
                      "levels": [{ "uuid": "lvl-vandal-std", "displayIcon": "https://img/vstd.png" }] },
                    { "uuid": "SKIN-PRIME", "displayName": "Prime Vandal", "contentTierUuid": "tier-premium",
                      "displayIcon": "https://img/prime.png",
                      "levels": [{ "uuid": "LVL-PRIME-1" }],
                      "chromas": [{ "uuid": "chroma-prime-blue", "displayName": "Prime Vandal Mavi",
                                    "fullRender": "https://img/prime-blue.png" }] }
                ]
            },
            {
                "uuid": CLASSIC, "displayName": "Classic", "category": "EEquippableCategory::Sidearm",
                "displayIcon": "https://img/classic.png", "defaultSkinUuid": "skin-classic-std",
                "skins": [{ "uuid": "skin-classic-std", "displayName": "Standart Classic", "chromas": [] }]
            }
        ]}),
        tiers: json!({ "data": [{ "uuid": "tier-premium", "highlightColor": "d1548db3", "rank": 2 }]}),
        maps: json!({ "data": [{ "mapUrl": "/Game/Maps/Ascent/Ascent", "displayName": "Ascent" }]}),
        buddies: json!({ "data": [{ "uuid": "b", "displayIcon": "https://img/buddy.png",
                                   "levels": [{ "uuid": "BUDDY-LVL" }] }]}),
        cards: json!({ "data": [{ "uuid": "card-1", "smallArt": "https://img/card-s.png", "wideArt": "https://img/card-w.png" }]}),
        titles: json!({ "data": [{ "uuid": "title-1", "titleText": "Efsane" }, { "uuid": "title-2", "titleText": null }]}),
        ranks: json!({ "data": [
            { "tiers": [{ "tier": 24, "tierName": "OLD" }] },
            { "tiers": [
                { "tier": 0, "tierName": "DERECESİZ" },
                { "tier": 12, "tierName": "ALTIN 1", "color": "ffd700ff", "smallIcon": "https://img/g1.png" },
                { "tier": 21, "tierName": "YÜCELİŞ 1", "smallIcon": "https://img/a1.png" },
                { "tier": 24, "tierName": "ÖLÜMSÜZLÜK 1", "smallIcon": "https://img/i1.png" }
            ]}
        ]}),
        contracts: json!({ "data": [
            { "uuid": "bp-1", "displayName": "Bölüm 9 Battle Pass",
              "content": { "relationType": "Season", "relationUuid": "ACT-NOW",
                           "chapters": [{ "levels": [{}, {}, {}] }, { "levels": [{}, {}] }] } },
            { "uuid": "agent-1", "displayName": "Jett Kontratı",
              "content": { "relationType": "Agent", "chapters": [{ "levels": [{}, {}, {}, {}] }] } }
        ]}),
        bundles: json!({ "data": [{ "uuid": "bundle-1", "displayName": "Prime", "displayIcon": "https://img/bundle.png" }]}),
        sprays: json!({ "data": [{ "uuid": "SPRAY-1", "displayName": "Kalp", "fullTransparentIcon": "https://img/heart.png",
                                  "levels": [{ "uuid": "spray-1-lvl" }] }]}),
        flex: json!({ "data": [{ "uuid": "flex-1", "displayName": "Kupa", "displayIcon": "https://img/cup.png" }]}),
        borders: json!({ "data": [
            { "uuid": "b20", "startingLevel": 20, "levelNumberAppearance": "https://img/b20.png" },
            { "uuid": "b1", "startingLevel": 1, "levelNumberAppearance": "https://img/b1.png" },
            { "uuid": "B-SPECIAL", "startingLevel": 500, "levelNumberAppearance": "https://img/bs.png" }
        ]}),
        missions: json!({ "data": [
            { "uuid": "M-1", "title": "3 maç oyna", "type": "EAresMissionType::Weekly", "xpGrant": 12000, "progressToComplete": 3 },
            { "uuid": "m-2", "displayName": "Günlük", "type": "EAresMissionType::Daily", "xpGrant": 2000, "progressToComplete": 10 }
        ]}),
    };
    Content::from_sources(&src)
}

fn item(skin: &str, chroma: Option<&str>, buddy: Option<&str>) -> Value {
    let mut sockets = json!({ SOCKET_SKIN: { "ID": SOCKET_SKIN, "Item": { "ID": skin, "TypeID": "x" } } });
    if let Some(ch) = chroma {
        sockets[SOCKET_CHROMA] = json!({ "ID": SOCKET_CHROMA, "Item": { "ID": ch, "TypeID": "y" } });
    }
    if let Some(b) = buddy {
        sockets[SOCKET_BUDDY] = json!({ "ID": SOCKET_BUDDY, "Item": { "ID": b, "TypeID": "z" } });
    }
    json!({ "ID": "w", "TypeID": "t", "Sockets": sockets })
}

#[test]
fn content_parses() {
    let c = content();
    assert_eq!(c.agents["aaaa-jett"].name, "Jett");
    assert_eq!(c.weapons[VANDAL].category, "Rifle");
    assert_eq!(c.skins["skin-prime"].tier.as_deref(), Some("tier-premium"));
    assert_eq!(c.skins["skin-prime"].weapon, VANDAL);
    assert_eq!(c.levels["lvl-prime-1"], "skin-prime");
    assert_eq!(c.skins["skin-vandal-std"].icon.as_deref(), Some("https://img/vstd.png"));
    assert_eq!(c.tiers["tier-premium"], "#d1548d");
    assert_eq!(c.tier_rank["tier-premium"], 2);
    assert_eq!(c.buddies["buddy-lvl"], "https://img/buddy.png");
    assert_eq!(c.titles.len(), 1);
    assert_eq!(c.rank(24).name, "ÖLÜMSÜZLÜK 1", "latest rank table wins");
    assert_eq!(c.rank(12).color.as_deref(), Some("#ffd700"));
    assert_eq!(c.contracts["bp-1"].levels, 5);
    assert_eq!(c.contracts["bp-1"].season.as_deref(), Some(ACT));
    assert_eq!(c.rank(5).name, "Seviye 5");
}

#[test]
fn skins_resolve_with_chroma_buddy_and_order() {
    let c = content();
    let items = json!({
        CLASSIC: item("skin-classic-std", None, None),
        VANDAL: item("skin-prime", Some("chroma-prime-blue"), Some("buddy-lvl")),
    });
    let w = weapon_skins(&items, &c);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].weapon_id, VANDAL, "rifles before sidearms");
    assert_eq!(w[0].skin, "Prime Vandal Mavi");
    assert_eq!(w[0].image.as_deref(), Some("https://img/prime-blue.png"));
    assert_eq!(w[0].tier_color.as_deref(), Some("#d1548d"));
    assert_eq!(w[0].buddy.as_deref(), Some("https://img/buddy.png"));
    assert!(!w[0].standard);
    assert!(w[1].standard);
    assert_eq!(w[1].buddy, None);
}

#[test]
fn core_game_match() {
    let c = content();
    let m = json!({
        "MapID": "/Game/Maps/Ascent/Ascent",
        "MatchmakingData": { "QueueID": "competitive" },
        "Players": [
            { "Subject": "enemy1", "TeamID": "Red", "CharacterID": "AAAA-SAGE",
              "PlayerIdentity": { "Incognito": true, "AccountLevel": 99, "HideAccountLevel": true } },
            { "Subject": "me", "TeamID": "Blue", "CharacterID": "aaaa-jett",
              "PlayerIdentity": { "Incognito": true, "AccountLevel": 120, "PlayerCardID": "CARD-1", "PlayerTitleID": "title-1" } },
            { "Subject": "mate", "TeamID": "Blue", "CharacterID": "aaaa-sage",
              "PlayerIdentity": { "Incognito": false, "AccountLevel": 50 } }
        ]
    });
    let l = json!({ "Loadouts": [
        { "CharacterID": "x", "Loadout": { "Subject": "enemy1", "Items": { VANDAL: item("skin-prime", None, None) } } },
        { "CharacterID": "x", "Loadout": { "Subject": "me", "Items": { VANDAL: item("skin-vandal-std", None, None) },
            "Sprays": { "SpraySelections": [{ "SocketID": "s", "SprayID": "spray-1", "LevelID": "x" }, { "SocketID": "t", "SprayID": "unknown" }] },
            "Expressions": { "AESSelections": [{ "SocketID": "u", "AssetID": "FLEX-1", "TypeID": "f" }, { "SocketID": "v", "AssetID": "SPRAY-1" }] } } }
    ]});
    let raw = players_from_core(&m);
    let by = loadouts_by_player(&l, &raw);
    let mut names = HashMap::new();
    names.insert("me".to_string(), ("Liqht".to_string(), "TR1".to_string()));
    names.insert("mate".to_string(), ("Dost".to_string(), "".to_string()));
    names.insert("enemy1".to_string(), ("ShouldNotShow".to_string(), "X".to_string()));
    let mut ranks_map = HashMap::new();
    ranks_map.insert("enemy1".to_string(), ranks::RankInfo { current: c.rank(12), ..Default::default() });
    let parties = number_parties(&[vec!["mate".to_string(), "me".to_string()]], &["enemy1".to_string(), "me".to_string(), "mate".to_string()]);
    let mut enc = HashMap::new();
    enc.insert("enemy1".to_string(), crate::history::Encounter {
        times: 2, ago: 3, ally: false, map: None, won: Some(true), agent: None, agent_icon: None,
        kills: 1, deaths: 2, assists: 3, acs: 100,
    });
    enc.insert("me".to_string(), crate::history::Encounter {
        times: 9, ago: 1, ally: true, map: None, won: None, agent: None, agent_icon: None,
        kills: 0, deaths: 0, assists: 0, acs: 0,
    });
    let x = Extras { names: &names, ranks: &ranks_map, parties: &parties, encounters: &enc };
    let p = assemble(&raw, &by, &c, "me", &x);

    assert_eq!(p.iter().map(|x| x.puuid.as_str()).collect::<Vec<_>>(), ["me", "mate", "enemy1"]);
    assert!(p[0].is_me && p[0].ally && !p[0].hidden, "own incognito flag never hides you from yourself");
    assert_eq!(p[0].name.as_deref(), Some("Liqht"));
    assert_eq!(p[0].card.as_deref(), Some("https://img/card-s.png"));
    assert_eq!(p[0].title.as_deref(), Some("Efsane"));
    let sprays: Vec<&str> = p[0].sprays.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(sprays, ["Kalp", "Kupa"], "unknown ids skipped, duplicates merged");
    assert_eq!(p[0].sprays[0].icon.as_deref(), Some("https://img/heart.png"));
    assert!(p[2].sprays.is_empty());
    assert_eq!(p[0].party, Some(1));
    assert_eq!(p[1].party, Some(1));
    assert_eq!(p[1].tag, None);
    assert!(p[1].weapons.is_empty());
    let enemy = &p[2];
    assert!(!enemy.ally && enemy.hidden);
    assert_eq!(enemy.name, None, "hidden names stay hidden");
    assert_eq!(enemy.level, None);
    assert_eq!(enemy.party, None);
    assert_eq!(enemy.rank.as_ref().map(|r| r.current.tier), Some(12));
    assert_eq!(enemy.agent.as_deref(), Some("Sage"));
    assert_eq!(enemy.weapons[0].skin, "Prime Vandal");
    assert_eq!(enemy.encounter.as_ref().map(|e| e.ago), Some(3));
    assert!(p[0].encounter.is_none(), "never an encounter with yourself");
    assert_eq!(p[0].border.as_deref(), Some("https://img/b20.png"), "level 120 earns the level-20 border");
    assert_eq!(weapon_meta(&p, &c).len(), 1);
}

#[test]
fn pregame_loadouts_without_subject_use_order() {
    let m = json!({ "QueueID": "unrated", "AllyTeam": { "Players": [
        { "Subject": "me", "CharacterID": "" },
        { "Subject": "mate", "CharacterID": "aaaa-jett" }
    ]}});
    let raw = players_from_pregame(&m);
    assert_eq!(raw[0].agent_id, None);
    let l = json!({ "Loadouts": [
        { "Items": { VANDAL: item("skin-vandal-std", None, None) } },
        { "Items": { VANDAL: item("skin-prime", None, None) } }
    ], "LoadoutsValid": true });
    let by = loadouts_by_player(&l, &raw);
    assert!(by.contains_key("me") && by.contains_key("mate"));
    let empty_n = HashMap::new();
    let empty_r = HashMap::new();
    let empty_p = HashMap::new();
    let empty_e = HashMap::new();
    let x = Extras { names: &empty_n, ranks: &empty_r, parties: &empty_p, encounters: &empty_e };
    let p = assemble(&raw, &by, &content(), "me", &x);
    assert!(p.iter().all(|x| x.ally));
    assert_eq!(p[1].weapons[0].skin, "Prime Vandal");
}

#[test]
fn parties_merge_and_number_in_roster_order() {
    let roster: Vec<String> = ["a", "b", "c", "d", "e", "f"].iter().map(|s| s.to_string()).collect();
    let groups = vec![
        vec!["d".to_string(), "e".to_string()],
        vec!["e".to_string(), "f".to_string()],
        vec!["a".to_string(), "b".to_string()],
        vec!["zz".to_string(), "c".to_string()],
    ];
    let n = number_parties(&groups, &roster);
    assert_eq!(n.get("a"), Some(&1));
    assert_eq!(n.get("b"), Some(&1));
    assert_eq!(n.get("c"), None, "outsider does not make a party");
    assert_eq!(n.get("d"), Some(&2));
    assert_eq!(n.get("f"), Some(&2));
}

#[test]
fn ranks_current_and_peak() {
    let c = content();
    let mmr = json!({
        "QueueSkills": { "competitive": { "SeasonalInfoBySeasonID": {
            "ACT-NOW": { "CompetitiveTier": 12, "RankedRating": 47, "NumberOfWinsWithPlacements": 9,
                         "NumberOfGames": 15, "WinsByTier": { "12": 3 } },
            "act-old": { "CompetitiveTier": 21, "WinsByTier": { "20": 2, "21": 1 } }
        }}}
    });
    let r = ranks::from_mmr(&mmr, Some(ACT), &c);
    assert_eq!(r.current.tier, 12);
    assert_eq!(r.rr, Some(47));
    assert_eq!(r.wins, Some(9));
    assert_eq!(r.games, Some(15));
    assert_eq!(r.peak.as_ref().map(|p| p.tier), Some(21));

    let fallback = json!({ "LatestCompetitiveUpdate": { "SeasonID": "act-now", "TierAfterUpdate": 24, "RankedRatingAfterUpdate": 10 } });
    let r = ranks::from_mmr(&fallback, Some(ACT), &c);
    assert_eq!(r.current.name, "ÖLÜMSÜZLÜK 1");
    assert_eq!(r.peak.map(|p| p.tier), Some(24));

    let none = ranks::from_mmr(&json!({}), Some(ACT), &c);
    assert_eq!(none.current.tier, 0);
    assert_eq!(none.rr, None);
    assert_eq!(none.peak, None);

    let content_v = json!({ "Seasons": [
        { "ID": "EP", "Type": "episode", "IsActive": true },
        { "ID": "ACT-NOW", "Type": "act", "IsActive": true },
        { "ID": "old", "Type": "act", "IsActive": false }
    ]});
    assert_eq!(ranks::active_act(&content_v).as_deref(), Some(ACT));
}

fn details() -> Value {
    json!({
        "matchInfo": { "matchId": "m1", "mapId": "/Game/Maps/Ascent/Ascent", "queueID": "competitive", "gameStartMillis": 1000 },
        "players": [
            { "subject": "me", "teamId": "Blue", "partyId": "p1", "characterId": "AAAA-JETT", "competitiveTier": 12,
              "stats": { "score": 5200, "roundsPlayed": 20, "kills": 20, "deaths": 10, "assists": 4 } },
            { "subject": "mate", "teamId": "Blue", "partyId": "p1", "characterId": "aaaa-sage",
              "stats": { "score": 3000, "roundsPlayed": 20, "kills": 10, "deaths": 15, "assists": 9 } },
            { "subject": "other", "teamId": "Red", "partyId": "p2",
              "stats": { "score": 1, "roundsPlayed": 20, "kills": 1, "deaths": 1, "assists": 1 } }
        ],
        "teams": [ { "teamId": "Blue", "won": true, "roundsWon": 13 }, { "teamId": "Red", "won": false, "roundsWon": 7 } ],
        "roundResults": [
            { "playerStats": [ { "subject": "me", "damage": [ { "headshots": 3, "bodyshots": 6, "legshots": 1 } ] } ] },
            { "playerStats": [ { "subject": "me", "damage": [ { "headshots": 1, "bodyshots": 9, "legshots": 0 } ] } ] }
        ]
    })
}

#[test]
fn match_stats() {
    let c = content();
    let d = details();
    let l = stats::line(&d, "me", &c).unwrap();
    assert_eq!(l.acs, 260);
    assert_eq!(l.hs, Some(20));
    assert_eq!(l.won, Some(true));
    assert_eq!((l.rounds_us, l.rounds_them), (13, 7));
    assert_eq!(l.agent.as_deref(), Some("Jett"));
    assert_eq!(l.map.as_deref(), Some("Ascent"));
    assert_eq!(l.queue, "Rekabetçi");
    assert_eq!(l.tier.as_ref().map(|t| t.tier), Some(12));
    assert!(stats::line(&d, "nobody", &c).is_none());

    let mut second = l.clone();
    second.won = Some(false);
    second.kills = 0;
    second.deaths = 10;
    second.acs = 100;
    second.hs = None;
    let s = stats::summarize(vec![l, second]);
    assert_eq!(s.kd, Some(1.0));
    assert_eq!(s.acs, Some(180));
    assert_eq!(s.hs, Some(20));
    assert_eq!(s.winrate, Some(50));
    assert_eq!(s.agents[0].count, 2);
    assert_eq!(stats::summarize(vec![]), stats::PlayerStats::default());

    let groups = stats::parties_in(&d, &["me".to_string(), "mate".to_string(), "other".to_string()]);
    assert_eq!(groups, vec![vec!["me".to_string(), "mate".to_string()]]);
}

#[test]
fn presences_and_friends() {
    use base64::Engine as _;
    let enc = |v: Value| base64::engine::general_purpose::STANDARD.encode(v.to_string());
    let p = json!({ "presences": [
        { "puuid": "f1", "product": "valorant", "state": "chat", "private": enc(json!({
            "sessionLoopState": "INGAME", "partyId": "P", "partySize": 2, "matchMap": "/Game/Maps/Ascent/Ascent",
            "queueId": "competitive", "competitiveTier": 12, "accountLevel": 77,
            "partyOwnerMatchScoreAllyTeam": 5, "partyOwnerMatchScoreEnemyTeam": 3 })) },
        { "puuid": "f2", "product": "valorant", "state": "chat", "private": enc(json!({
            "matchPresenceData": { "sessionLoopState": "MENUS" },
            "partyPresenceData": { "partyId": "P", "partySize": 2 },
            "playerPresenceData": { "competitiveTier": 0 } })) },
        { "puuid": "f3", "product": "league_of_legends", "state": "chat", "private": "<xml/>" }
    ]});
    let pres = social::parse_presences(&p);
    assert_eq!(pres.len(), 3);
    assert_eq!(pres[0].score(), Some([5, 3]));
    assert_eq!(pres[1].party_id().as_deref(), Some("P"), "nested presence format");
    assert_eq!(social::party_groups(&pres), vec![vec!["f1".to_string(), "f2".to_string()]]);

    let friends = json!({ "friends": [
        { "puuid": "f4", "game_name": "Uyuyan", "game_tag": "1" },
        { "puuid": "f3", "game_name": "Lolcu", "game_tag": "2" },
        { "puuid": "f2", "game_name": "Menu", "game_tag": "3" },
        { "puuid": "f1", "game_name": "Maci", "game_tag": "4" }
    ]});
    let list = social::friends(&friends, &pres, &content());
    let order: Vec<&str> = list.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(order, ["Maci", "Menu", "Lolcu", "Uyuyan"]);
    assert_eq!(list[0].status, "Maçta");
    assert_eq!(list[0].map.as_deref(), Some("Ascent"));
    assert_eq!(list[0].score, Some([5, 3]));
    assert_eq!(list[0].rank.as_ref().map(|r| r.tier), Some(12));
    assert_eq!(list[0].level, Some(77));
    assert_eq!(list[1].rank, None);
    assert_eq!(list[2].status, "League of Legends'ta");
    assert!(!list[3].online);
}

#[test]
fn account_parts() {
    let c = content();
    let w = account::wallet(&json!({ "Balances": { VP: 1250, RP: 40 } }));
    assert_eq!((w.vp, w.rp, w.kc), (Some(1250), Some(40), None));

    let store = account::store(
        &json!({
            "SkinsPanelLayout": {
                "SingleItemStoreOffers": [{ "OfferID": "o", "Cost": { VP: 1775 }, "Rewards": [{ "ItemID": "LVL-PRIME-1" }] }],
                "SingleItemOffersRemainingDurationInSeconds": 3600
            },
            "BonusStore": {
                "BonusStoreOffers": [{ "Offer": { "Cost": { VP: 1775 }, "Rewards": [{ "ItemID": "lvl-prime-1" }] },
                                       "DiscountPercent": 30, "DiscountCosts": { VP: 1242 } }],
                "BonusStoreRemainingDurationInSeconds": 99
            },
            "FeaturedBundle": { "Bundle": { "DataAssetID": "BUNDLE-1", "DurationRemainingInSeconds": 500 } }
        }),
        &c,
    );
    assert_eq!(store.daily[0].name, "Prime Vandal");
    assert_eq!(store.daily[0].price, Some(1775));
    assert_eq!(store.daily_ends_in, Some(3600));
    assert_eq!(store.night[0].price, Some(1242));
    assert_eq!(store.night[0].original, Some(1775));
    assert_eq!(store.night[0].discount, Some(30));
    assert_eq!(store.bundles[0].name, "Prime");

    let owned = account::collection(
        &json!({ "Entitlements": [{ "ItemID": "LVL-PRIME-1" }, { "ItemID": "lvl-prime-1" }, { "ItemID": "lvl-vandal-std" }, { "ItemID": "unknown" }] }),
        &c,
    );
    assert_eq!(owned.len(), 1, "dedupes levels and skips default skins");
    assert_eq!(owned[0].weapon, "Vandal");

    let contracts = account::contracts(
        &json!({ "Contracts": [
            { "ContractDefinitionID": "BP-1", "ProgressionLevelReached": 3 },
            { "ContractDefinitionID": "agent-1", "ProgressionLevelReached": 1 }
        ], "ActiveSpecialContract": "AGENT-1" }),
        Some(ACT),
        &c,
    );
    assert_eq!(contracts.len(), 2);
    assert_eq!((contracts[0].level, contracts[0].total), (3, 5));
    assert_eq!(contracts[1].name, "Jett Kontratı");
    assert_eq!((contracts[1].level, contracts[1].total), (1, 4));

    let rr = account::rr_history(
        &json!({ "Matches": [{ "MapID": "/Game/Maps/Ascent/Ascent", "TierAfterUpdate": 12, "RankedRatingAfterUpdate": 40, "RankedRatingEarned": -17, "MatchStartTime": 5 }] }),
        &c,
    );
    assert_eq!(rr[0].earned, -17);
    assert_eq!(rr[0].map.as_deref(), Some("Ascent"));
}

#[test]
fn queue_names() {
    assert_eq!(stats::queue_label("competitive"), "Rekabetçi");
    assert_eq!(stats::queue_label("newmode"), "Newmode");
    assert_eq!(stats::queue_label(""), "Özel oyun");
}


#[test]
fn borders_and_missions() {
    let c = content();
    assert_eq!(c.border(None, Some(5)).as_deref(), Some("https://img/b1.png"));
    assert_eq!(c.border(Some("b-special"), Some(5)).as_deref(), Some("https://img/bs.png"), "chosen border wins");
    assert_eq!(c.border(None, None), None);

    let v = json!({
        "Missions": [
            { "ID": "m-1", "Objectives": { "o": 2 }, "Complete": false, "ExpirationTime": "2026-10-20T00:00:00Z" },
            { "ID": "M-2", "Objectives": { "o": 15 }, "Complete": true },
            { "ID": "unknown", "Objectives": {}, "Complete": false }
        ]
    });
    let m = account::missions(&v, &c);
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].kind, "weekly", "open missions first");
    assert_eq!((m[0].progress, m[0].target, m[0].xp), (2, 3, 12000));
    assert_eq!(m[0].expires.as_deref(), Some("2026-10-20T00:00:00Z"));
    assert_eq!(m[1].title, "Görev");
    let done = m.iter().find(|x| x.complete).unwrap();
    assert_eq!((done.progress, done.target), (10, 10), "progress is capped at the target");
}

#[test]
fn own_history_encounters_and_scoreboard() {
    use crate::history;
    let c = content();
    let older = json!({
        "matchInfo": { "matchId": "m0", "mapId": "/Game/Maps/Ascent/Ascent", "queueID": "unrated" },
        "players": [
            { "subject": "me", "teamId": "Red", "stats": { "score": 1, "roundsPlayed": 1, "kills": 0, "deaths": 0, "assists": 0 } },
            { "subject": "foe", "teamId": "Red", "characterId": "aaaa-sage",
              "stats": { "score": 200, "roundsPlayed": 1, "kills": 9, "deaths": 9, "assists": 9 } }
        ],
        "teams": [ { "teamId": "Red", "won": false, "roundsWon": 1 }, { "teamId": "Blue", "won": true, "roundsWon": 13 } ]
    });
    let newest = history::own_match(&details(), "me", &c).unwrap();
    let old = history::own_match(&older, "me", &c).unwrap();
    assert_eq!(newest.my_won, Some(true));
    let list = vec![newest, old];

    let e = history::encounter(&list, "other", &c).unwrap();
    assert_eq!((e.ago, e.times, e.ally), (1, 1, false));
    assert_eq!(e.map.as_deref(), Some("Ascent"));
    let f = history::encounter(&list, "foe", &c).unwrap();
    assert_eq!((f.ago, f.ally, f.won), (2, true, Some(false)));
    assert_eq!(f.agent.as_deref(), Some("Sage"));
    assert!(history::encounter(&list, "stranger", &c).is_none());

    let mut friends = std::collections::HashSet::new();
    friends.insert("nobody".to_string());
    let mut d = details();
    d["players"][0]["gameName"] = json!("Liqht");
    d["players"][1]["gameName"] = json!("Kanka");
    d["players"][2]["gameName"] = json!("Yabanci");
    d["roundResults"][0]["playerStats"][0]["damage"][0]["damage"] = json!(300);
    d["matchInfo"]["gameLengthMillis"] = json!(1_980_000);
    let sb = history::scoreboard(&d, "me", &c, &friends).unwrap();
    assert_eq!(sb.minutes, 33);
    assert_eq!((sb.won, sb.rounds_us, sb.rounds_them), (Some(true), 13, 7));
    assert_eq!(sb.mine.len(), 2);
    assert_eq!(sb.mine[0].name.as_deref(), Some("Liqht"));
    assert_eq!(sb.mine[0].adr, 15);
    assert_eq!(sb.mine[1].name.as_deref(), Some("Kanka"), "same party shows names");
    assert_eq!(sb.theirs[0].name, None, "strangers stay anonymous");
}

#[test]
fn inventory_value() {
    use crate::inventory as inv;
    let c = content();
    let owned = inv::by_type(&json!({ "EntitlementsByTypes": [
        { "ItemTypeID": inv::T_SKIN, "Entitlements": [{ "ItemID": "LVL-PRIME-1" }, { "ItemID": "lvl-vandal-std" }] },
        { "ItemTypeID": inv::T_BUDDY, "Entitlements": [{ "ItemID": "buddy-lvl" }, { "ItemID": "buddy-lvl" }] },
        { "ItemTypeID": inv::T_SPRAY, "Entitlements": [{ "ItemID": "spray-1" }] },
        { "ItemTypeID": inv::T_TITLE, "Entitlements": [{ "ItemID": "title-1" }] },
        { "ItemTypeID": inv::T_AGENT, "Entitlements": [{ "ItemID": "aaaa-jett" }, { "ItemID": "not-an-agent" }] }
    ]}));
    let prices = inv::prices(&json!({ "Offers": [
        { "Cost": { VP: 1775 }, "Rewards": [{ "ItemID": "lvl-prime-1" }] },
        { "Cost": { VP: 475 }, "Rewards": [{ "ItemID": "BUDDY-LVL" }] },
        { "Cost": { "other-currency": 5 }, "Rewards": [{ "ItemID": "spray-1" }] }
    ]}));
    let r = inv::build(&owned, &prices, &c, false);
    assert!(!r.estimated);
    let guess = inv::build(&owned, &std::collections::HashMap::new(), &c, true);
    assert_eq!(guess.groups[0].items[0].price, None, "test tier is not a standard tier");
    assert!(!guess.estimated);
    let g = |k: &str| r.groups.iter().find(|g| g.key == k).unwrap();
    assert_eq!(g("skins").items.len(), 1, "standard skin skipped");
    assert_eq!(g("skins").items[0].price, Some(1775));
    assert_eq!(g("buddies").items.len(), 1, "duplicates merged");
    assert_eq!(g("sprays").items[0].price, None);
    assert_eq!(g("titles").items[0].name, "Efsane");
    assert_eq!(r.value, 1775 + 475);
    assert_eq!((r.priced, r.unpriced), (2, 2));
    assert_eq!(r.agents, 1);
}

#[test]
fn party_lobby() {
    let c = content();
    let party = json!({
        "State": "MATCHMAKING",
        "MatchmakingData": { "QueueID": "competitive" },
        "Members": [
            { "Subject": "mate", "CompetitiveTier": 12, "IsOwner": true, "IsReady": true,
              "PlayerIdentity": { "AccountLevel": 30, "PlayerCardID": "card-1" } },
            { "Subject": "me", "CompetitiveTier": 0, "IsReady": true,
              "PlayerIdentity": { "AccountLevel": 120, "PreferredLevelBorderID": "B-SPECIAL", "PlayerTitleID": "title-1" } }
        ]
    });
    let mut names = HashMap::new();
    names.insert("mate".to_string(), ("Kanka".to_string(), "0001".to_string()));
    let l = lobby(&party, "me", &c, &names);
    assert!(l.searching);
    assert_eq!(l.queue, "Rekabetçi");
    assert_eq!(l.members[0].puuid, "me", "you first");
    assert_eq!(l.members[0].border.as_deref(), Some("https://img/bs.png"));
    assert_eq!(l.members[0].title.as_deref(), Some("Efsane"));
    assert_eq!(l.members[1].name.as_deref(), Some("Kanka"));
    assert!(l.members[1].owner);
    assert_eq!(l.members[1].rank.tier, 12);
    assert_eq!(l.members[1].card.as_deref(), Some("https://img/card-s.png"));
}

#[test]
fn update_versions() {
    use crate::updater;
    assert!(updater::is_newer("v0.4.0", "0.3.0"));
    assert!(updater::is_newer("1.0", "0.9.9"));
    assert!(!updater::is_newer("v0.3.0", "0.3.0"));
    assert!(!updater::is_newer("garbage", "0.3.0"));
    let rel = json!({ "tag_name": "v0.4.0", "body": "notlar", "assets": [
        { "name": "ValoLiqht-setup.exe", "browser_download_url": "https://github.com/x/setup" },
        { "name": "ValoLiqht-portable.exe", "browser_download_url": "https://github.com/x/portable", "digest": "sha256:ab" }
    ]});
    let u = updater::from_release(&rel, "0.3.0").unwrap();
    assert_eq!(u.version, "0.4.0");
    assert_eq!(u.url, "https://github.com/x/portable");
    assert!(updater::from_release(&rel, "0.4.0").is_none());
    let hello = b"hello";
    assert!(updater::verify(hello, Some("sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")).is_ok());
    assert!(updater::verify(hello, Some("sha256:00")).is_err());
    assert!(updater::verify(hello, None).is_ok());
}


#[test]
fn prices_from_storefront_shapes() {
    use crate::inventory as inv;
    let front = json!({
        "FeaturedBundle": { "Bundles": [{ "Items": [
            { "Item": { "ItemTypeID": "x", "ItemID": "LVL-A", "Amount": 1 }, "BasePrice": 1775, "CurrencyID": VP },
            { "Item": { "ItemID": "free" }, "BasePrice": 0, "CurrencyID": VP }
        ]}]},
        "PluginStores": [{ "PluginOffers": { "StoreOffers": [{
            "PurchaseInformation": { "Cost": { VP: 3550 }, "Rewards": [] },
            "SubOffers": [
                { "PurchaseInformation": { "Cost": { VP: 2175 }, "Rewards": [{ "ItemID": "lvl-b", "Quantity": 1 }] } },
                { "PurchaseInformation": { "Cost": { VP: 475 }, "Rewards": [{ "ItemID": "buddy-x", "Quantity": 2 }] } }
            ]
        }]}}],
        "AccessoryStore": { "AccessoryStoreOffers": [{ "Offer": { "Cost": { "kc": 4000 }, "Rewards": [{ "ItemID": "card-k" }] } }] },
        "Bundle": { "Cost": { VP: 9000 }, "Rewards": [{ "ItemID": "a" }, { "ItemID": "b" }] }
    });
    let p = inv::prices(&front);
    assert_eq!(p.get("lvl-a"), Some(&1775));
    assert_eq!(p.get("lvl-b"), Some(&2175));
    assert_eq!(p.get("buddy-x"), Some(&475));
    assert!(!p.contains_key("free"));
    assert!(!p.contains_key("card-k"), "only VP prices");
    assert!(!p.contains_key("a"), "multi-item rewards are not per-item prices");
}
