//! Sway's IPC through `swaymsg`. Sway has no socket protocol as convenient as Hyprland's, so
//! these are the CLI forms documented for scripts.

use std::process::Command;

pub fn focused_monitor() -> Option<String> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_outputs", "-r"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_focused(&String::from_utf8_lossy(&output.stdout))
}

/// The `name` of the output with `"focused": true`.
pub fn parse_focused(json: &str) -> Option<String> {
    let outputs = serde_json::from_str::<serde_json::Value>(json).ok()?;
    outputs
        .as_array()?
        .iter()
        .find(|o| o.get("focused").and_then(serde_json::Value::as_bool) == Some(true))
        .and_then(|o| o.get("name")?.as_str().map(str::to_owned))
}

pub fn version() -> Option<String> {
    let output = Command::new("sway").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    let rest = line.strip_prefix("sway version ")?;
    Some(rest.split_whitespace().next()?.to_owned())
}

pub fn exit_command() -> String {
    "swaymsg exit".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_focused_output() {
        let json = r#"[
            {"name":"HDMI-A-1","focused":false,"active":true,"rect":{"x":1920,"y":0,"width":1920,"height":1080}},
            {"name":"eDP-1","focused":true,"active":true,"rect":{"x":0,"y":0,"width":1920,"height":1080}}
        ]"#;
        assert_eq!(parse_focused(json).as_deref(), Some("eDP-1"));
    }

    #[test]
    fn a_missing_or_broken_answer_is_none() {
        assert_eq!(parse_focused("not json"), None);
        assert_eq!(parse_focused("[]"), None);
        assert_eq!(parse_focused(r#"[{"name":"eDP-1","focused":false}]"#), None);
    }
}
