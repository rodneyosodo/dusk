//! Hyprland's IPC socket: monitor focus and the exit dispatcher. No subprocess for `j/monitors`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub name: String,
    pub focused: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

fn signature() -> Option<String> {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .ok()
        .filter(|s| !s.is_empty())
}

fn socket_path() -> Option<PathBuf> {
    let sig = signature()?;
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime)
            .join("hypr")
            .join(&sig)
            .join(".socket.sock");
        if p.exists() {
            return Some(p);
        }
    }
    let p = PathBuf::from("/tmp/hypr").join(&sig).join(".socket.sock");
    p.exists().then_some(p)
}

fn request(command: &str) -> Option<String> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(path).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(600)))
        .ok()?;
    stream
        .set_write_timeout(Some(Duration::from_millis(600)))
        .ok()?;
    stream.write_all(command.as_bytes()).ok()?;
    stream.flush().ok()?;
    let mut out = String::new();
    stream.read_to_string(&mut out).ok()?;
    Some(out)
}

/// Parse `j/monitors`, kept separate so the IPC shape has a unit test.
pub fn parse_monitors(json: &str) -> Vec<Monitor> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(array) = value.as_array() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|m| {
            Some(Monitor {
                name: m.get("name")?.as_str()?.to_owned(),
                focused: m
                    .get("focused")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                x: m.get("x").and_then(serde_json::Value::as_i64).unwrap_or(0) as i32,
                y: m.get("y").and_then(serde_json::Value::as_i64).unwrap_or(0) as i32,
                width: m
                    .get("width")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32,
                height: m
                    .get("height")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32,
                scale: m
                    .get("scale")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0),
            })
        })
        .collect()
}

pub fn monitors() -> Vec<Monitor> {
    request("j/monitors").map_or_else(Vec::new, |json| parse_monitors(&json))
}

pub fn focused_monitor() -> Option<String> {
    monitors().into_iter().find(|m| m.focused).map(|m| m.name)
}

/// `0.57.0` from `j/version`, or the first number pair in `hyprctl version` as a fallback.
pub fn version() -> Option<String> {
    static VERSION: OnceLock<Option<String>> = OnceLock::new();
    VERSION
        .get_or_init(|| {
            if let Some(value) = request("j/version")
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            {
                if let Some(tag) = value.get("tag").and_then(serde_json::Value::as_str) {
                    let tag = tag.trim_start_matches('v');
                    if !tag.is_empty() && tag != "unknown" {
                        return Some(tag.to_owned());
                    }
                }
                if let Some(commit) = value.get("commit").and_then(serde_json::Value::as_str)
                    && !commit.is_empty()
                    && commit != "unknown"
                {
                    return Some(commit.chars().take(7).collect());
                }
            }
            let out = std::process::Command::new("hyprctl")
                .arg("version")
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            let line = text.lines().next().unwrap_or_default();
            let rest = line.strip_prefix("Hyprland ")?;
            Some(rest.split_whitespace().next().unwrap_or("?").to_owned())
        })
        .clone()
}

fn version_at_least(major: u32, minor: u32) -> bool {
    let Some(v) = version() else { return false };
    let mut parts = v.split('.');
    let a = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let b = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    (a, b) >= (major, minor)
}

/// Hyprland 0.57 moved dispatchers to Lua; older releases still take `hyprctl dispatch exit`.
pub fn exit_command() -> String {
    if version_at_least(0, 57) {
        "hyprctl dispatch 'hl.dsp.exit()'".into()
    } else {
        "hyprctl dispatch exit".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_focused_monitor_and_skips_bad_entries() {
        let json = r#"[
            {"name":"eDP-1","x":0,"y":0,"width":1920,"height":1080,"scale":1.0,"focused":false},
            {"name":"DP-1","x":1920,"y":-200,"width":2560,"height":1440,"scale":1.5,"focused":true},
            {"name": 12}
        ]"#;
        let monitors = parse_monitors(json);
        assert_eq!(monitors.len(), 2);
        assert_eq!(monitors[1].name, "DP-1");
        assert_eq!(monitors[1].x, 1920);
        assert_eq!(monitors[1].y, -200);
        assert!(monitors[1].focused);
        assert_eq!(monitors[1].scale, 1.5);
    }

    #[test]
    fn a_broken_json_answer_is_empty_not_a_panic() {
        assert!(parse_monitors("not json").is_empty());
        assert!(parse_monitors("{}").is_empty());
    }
}
