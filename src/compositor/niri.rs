//! Niri's IPC through `niri msg --json`. The JSON output is the stable interface.

use std::process::Command;

pub fn focused_monitor() -> Option<String> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-output"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_focused(&String::from_utf8_lossy(&output.stdout))
}

/// `niri msg --json focused-output` prints the `Output` object. A raw socket reply wraps it in
/// `{"Ok":{"FocusedOutput":{...}}}`; accept both.
pub fn parse_focused(json: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
    let value = value.get("Ok").unwrap_or(&value);
    let output = value.get("FocusedOutput").unwrap_or(value);
    output.get("name")?.as_str().map(str::to_owned)
}

pub fn version() -> Option<String> {
    let output = Command::new("niri").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    Some(
        line.strip_prefix("niri ")?
            .split_whitespace()
            .next()?
            .to_owned(),
    )
}

pub fn exit_command() -> String {
    // `quit` asks for confirmation in the GUI; the CLI flag is documented.
    "niri msg action quit --skip-confirmation".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_focused_output_object() {
        let json = r#"{"name":"eDP-1","make":"AU Optronics","model":"0xA08B","logical":{"x":0,"y":0,"width":1920,"height":1080,"scale":1.0,"transform":"Normal"}}"#;
        assert_eq!(parse_focused(json).as_deref(), Some("eDP-1"));
    }

    #[test]
    fn also_reads_a_raw_socket_reply() {
        let json = r#"{"Ok":{"FocusedOutput":{"name":"DP-1"}}}"#;
        assert_eq!(parse_focused(json).as_deref(), Some("DP-1"));
    }

    #[test]
    fn a_broken_answer_is_none() {
        assert_eq!(parse_focused("not json"), None);
        assert_eq!(parse_focused("{}"), None);
    }
}
