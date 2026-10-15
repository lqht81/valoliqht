//! Files the Riot Client and VALORANT leave on this PC.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Lockfile {
    pub port: u16,
    pub password: String,
}

/// `name:pid:port:password:protocol`
pub fn parse_lockfile(text: &str) -> Option<Lockfile> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.len() < 5 {
        return None;
    }
    Some(Lockfile {
        port: parts[2].parse().ok()?,
        password: parts[3].to_string(),
    })
}

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

pub fn read_lockfile() -> Option<Lockfile> {
    let path = local_app_data()?
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile");
    parse_lockfile(&std::fs::read_to_string(path).ok()?)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LogInfo {
    pub region: Option<String>,
    pub shard: Option<String>,
    pub version: Option<String>,
}

fn plain_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Pull the game server host (`glz-<region>-1.<shard>.a.pvp.net`) and the
/// client build out of ShooterGame.log. The last occurrence wins.
pub fn parse_log(text: &str) -> LogInfo {
    let mut info = LogInfo::default();

    let mut rest = text;
    while let Some(i) = rest.find("https://glz-") {
        let after = &rest[i + "https://glz-".len()..];
        if let Some(end) = after.find(".a.pvp.net") {
            if let Some((r, s)) = after[..end].split_once('.') {
                if let Some(r) = r.strip_suffix("-1") {
                    if plain_token(r) && plain_token(s) {
                        info.region = Some(r.to_string());
                        info.shard = Some(s.to_string());
                    }
                }
            }
        }
        rest = after;
    }

    const VERSION_KEY: &str = "CI server version: ";
    if let Some(i) = text.rfind(VERSION_KEY) {
        let v: String = text[i + VERSION_KEY.len()..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if v.starts_with("release-") {
            info.version = Some(v);
        }
    }
    info
}

pub fn read_log() -> LogInfo {
    let Some(base) = local_app_data() else {
        return LogInfo::default();
    };
    let path = base
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");
    match std::fs::read(path) {
        Ok(bytes) => parse_log(&String::from_utf8_lossy(&bytes)),
        Err(_) => LogInfo::default(),
    }
}

/// Shard for a deployment name from the launch arguments.
pub fn shard_for_region(region: &str) -> &'static str {
    match region {
        "na" | "latam" | "br" | "pbe" => "na",
        "ap" => "ap",
        "kr" => "kr",
        _ => "eu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile() {
        let lf = parse_lockfile("Riot Client:1234:51234:AbC_dEf-123:https\n").unwrap();
        assert_eq!(lf.port, 51234);
        assert_eq!(lf.password, "AbC_dEf-123");
        assert!(parse_lockfile("broken").is_none());
    }

    #[test]
    fn log() {
        let text = "[x] https://glz-na-1.na.a.pvp.net/old\n\
                    LogShooter: CI server version: release-11.07-shipping-6-3812345\n\
                    [y] GET https://glz-eu-1.eu.a.pvp.net/pregame/v1/players/abc\n";
        let info = parse_log(text);
        assert_eq!(info.region.as_deref(), Some("eu"));
        assert_eq!(info.shard.as_deref(), Some("eu"));
        assert_eq!(info.version.as_deref(), Some("release-11.07-shipping-6-3812345"));
    }

    #[test]
    fn log_empty() {
        assert_eq!(parse_log("nothing here"), LogInfo::default());
    }
}
