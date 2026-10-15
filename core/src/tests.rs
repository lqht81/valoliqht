use super::*;
use serde_json::json;

const VANDAL: &str = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const CLASSIC: &str = "29a0cfab-485b-f5d5-779a-b59f85e204a8";

fn content() -> Content {
    let agents = json!({ "data": [
        { "uuid": "AAAA-JETT", "displayName": "Jett", "displayIconSmall": "https://img/jett.png" },
        { "uuid": "aaaa-sage", "displayName": "Sage", "displayIcon": "https://img/sage.png" }
    ]});
    let weapons = json!({ "data": [
        {
            "uuid": VANDAL, "displayName": "Vandal", "category": "EEquippableCategory::Rifle",
            "displayIcon": "https://img/vandal.png", "defaultSkinUuid": "skin-vandal-std",
            "skins": [
                { "uuid": "skin-vandal-std", "displayName": "Standart Vandal", "contentTierUuid": null,
                  "displayIcon": null, "chromas": [], "levels": [{ "displayIcon": "https://img/vstd.png" }] },
                { "uuid": "SKIN-PRIME", "displayName": "Prime Vandal", "contentTierUuid": "tier-premium",
                  "displayIcon": "https://img/prime.png",
                  "chromas": [{ "uuid": "chroma-prime-blue", "displayName": "Prime Vandal Mavi",
                                "fullRender": "https://img/prime-blue.png" }] }
            ]
        },
        {
            "uuid": CLASSIC, "displayName": "Classic", "category": "EEquippableCategory::Sidearm",
            "displayIcon": "https://img/classic.png", "defaultSkinUuid": "skin-classic-std",
            "skins": [{ "uuid": "skin-classic-std", "displayName": "Standart Classic", "chromas": [] }]
        }
    ]});
    let tiers = json!({ "data": [{ "uuid": "tier-premium", "highlightColor": "d1548db3" }]});
    let maps = json!({ "data": [{ "mapUrl": "/Game/Maps/Ascent/Ascent", "displayName": "Ascent" }]});
    Content::from_values(&agents, &weapons, &tiers, &maps)
}

fn item(skin: &str, chroma: Option<&str>) -> Value {
    let mut sockets = json!({ SOCKET_SKIN: { "ID": SOCKET_SKIN, "Item": { "ID": skin, "TypeID": "x" } } });
    if let Some(ch) = chroma {
        sockets[SOCKET_CHROMA] = json!({ "ID": SOCKET_CHROMA, "Item": { "ID": ch, "TypeID": "y" } });
    }
    json!({ "ID": "w", "TypeID": "t", "Sockets": sockets })
}

#[test]
fn content_parses() {
    let c = content();
    assert_eq!(c.agents["aaaa-jett"].name, "Jett");
    assert_eq!(c.weapons[VANDAL].category, "Rifle");
    assert_eq!(c.skins["skin-prime"].tier.as_deref(), Some("tier-premium"));
    assert_eq!(c.skins["skin-vandal-std"].icon.as_deref(), Some("https://img/vstd.png"));
    assert_eq!(c.tiers["tier-premium"], "#d1548d");
    assert_eq!(c.maps["/Game/Maps/Ascent/Ascent"], "Ascent");
}

#[test]
fn skins_resolve_with_chroma_and_order() {
    let c = content();
    let items = json!({
        CLASSIC: item("skin-classic-std", None),
        VANDAL: item("skin-prime", Some("chroma-prime-blue")),
    });
    let w = weapon_skins(&items, &c);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].weapon_id, VANDAL, "rifles before sidearms");
    assert_eq!(w[0].skin, "Prime Vandal Mavi");
    assert_eq!(w[0].image.as_deref(), Some("https://img/prime-blue.png"));
    assert_eq!(w[0].tier_color.as_deref(), Some("#d1548d"));
    assert!(!w[0].standard);
    assert!(w[1].standard);
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
              "PlayerIdentity": { "Incognito": true, "AccountLevel": 120 } },
            { "Subject": "mate", "TeamID": "Blue", "CharacterID": "aaaa-sage",
              "PlayerIdentity": { "Incognito": false, "AccountLevel": 50 } }
        ]
    });
    let l = json!({ "Loadouts": [
        { "CharacterID": "x", "Loadout": { "Subject": "enemy1", "Items": { VANDAL: item("skin-prime", None) } } },
        { "CharacterID": "x", "Loadout": { "Subject": "me", "Items": { VANDAL: item("skin-vandal-std", None) } } }
    ]});
    let raw = players_from_core(&m);
    let by = loadouts_by_player(&l, &raw);
    let mut names = HashMap::new();
    names.insert("me".to_string(), ("Liqht".to_string(), "TR1".to_string()));
    names.insert("mate".to_string(), ("Dost".to_string(), "".to_string()));
    names.insert("enemy1".to_string(), ("ShouldNotShow".to_string(), "X".to_string()));
    let p = assemble(&raw, &by, &c, "me", &names);

    assert_eq!(p.iter().map(|x| x.puuid.as_str()).collect::<Vec<_>>(), ["me", "mate", "enemy1"]);
    assert!(p[0].is_me && p[0].ally && !p[0].hidden, "own incognito flag never hides you from yourself");
    assert_eq!(p[0].name.as_deref(), Some("Liqht"));
    assert_eq!(p[1].tag, None);
    assert!(p[1].weapons.is_empty());
    let enemy = &p[2];
    assert!(!enemy.ally && enemy.hidden);
    assert_eq!(enemy.name, None, "hidden names stay hidden");
    assert_eq!(enemy.level, None);
    assert_eq!(enemy.agent.as_deref(), Some("Sage"));
    assert_eq!(enemy.weapons[0].skin, "Prime Vandal");
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
        { "Items": { VANDAL: item("skin-vandal-std", None) } },
        { "Items": { VANDAL: item("skin-prime", None) } }
    ], "LoadoutsValid": true });
    let by = loadouts_by_player(&l, &raw);
    assert!(by.contains_key("me") && by.contains_key("mate"));
    let p = assemble(&raw, &by, &content(), "me", &HashMap::new());
    assert!(p.iter().all(|x| x.ally));
    assert_eq!(p[1].weapons[0].skin, "Prime Vandal");
}

#[test]
fn queue_names() {
    assert_eq!(queue_label("competitive"), "Rekabetçi");
    assert_eq!(queue_label("newmode"), "Newmode");
    assert_eq!(queue_label(""), "Özel oyun");
}
