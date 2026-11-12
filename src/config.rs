//! KDL config: one readable file, parsed so a mistake is a line-numbered problem rather than
//! a reason the menu cannot open. Actions that parse survive a bad colour or unknown node.

use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode};

use crate::problem::{Problem, node_line};
use crate::theme::{Appearance, Layout, Theme, config_dir};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionPatch {
    pub id: String,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub run: Option<String>,
    pub key: Option<Option<char>>,
    pub confirm: Option<bool>,
    pub line: usize,
}

impl ActionPatch {
    pub fn new(id: String, line: usize) -> Self {
        Self {
            id,
            label: None,
            icon: None,
            run: None,
            key: None,
            confirm: None,
            line,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub theme: Theme,
    /// None means the theme's default layout.
    pub layout: Option<Layout>,
    pub appearance: Appearance,
    pub hide: Vec<String>,
    pub order: Vec<String>,
    pub actions: Vec<ActionPatch>,
    pub problems: Vec<Problem>,
    pub source: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            layout: None,
            appearance: Appearance::Auto,
            hide: Vec::new(),
            order: Vec::new(),
            actions: Vec::new(),
            problems: Vec::new(),
            source: None,
        }
    }
}

impl Config {
    pub fn effective_layout(&self) -> Layout {
        self.layout.unwrap_or(self.theme.layout)
    }

    /// `$XDG_CONFIG_HOME/dusk/config.kdl`, or `~/.config/dusk/config.kdl`.
    pub fn default_path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("config.kdl"))
    }

    /// Loads `path`, or the default config when `path` is None. A missing file is not a problem:
    /// no config is the normal case.
    pub fn load(path: Option<PathBuf>) -> Self {
        let path = path.or_else(Self::default_path);
        let Some(path) = path else {
            return Self::default();
        };
        if !path.exists() {
            return Config {
                source: Some(path),
                ..Self::default()
            };
        }
        match std::fs::read_to_string(&path) {
            Ok(source) => {
                let beside = path.parent().map(Path::to_path_buf);
                let mut config =
                    Self::from_source(&source, &path.display().to_string(), beside.as_deref());
                config.source = Some(path);
                config
            }
            Err(err) => {
                let mut config = Self::default();
                config.problems.push(Problem::new(
                    path.display().to_string(),
                    1,
                    format!("cannot read config: {err}"),
                ));
                config.source = Some(path);
                config
            }
        }
    }

    /// Parses `source`; `file` is what problems name, and `beside` is where a theme path is
    /// looked up (`themes/<id>.kdl` next to the config).
    pub fn from_source(source: &str, file: &str, beside: Option<&Path>) -> Self {
        let mut config = Config {
            source: beside.map(|p| p.join("config.kdl")),
            ..Config::default()
        };
        let doc: KdlDocument = match source.parse() {
            Ok(doc) => doc,
            Err(err) => {
                let line = err
                    .diagnostics
                    .first()
                    .map_or(1, |d| crate::problem::line_at(source, d.span.offset()));
                config
                    .problems
                    .push(Problem::new(file, line, "this file is not valid KDL"));
                return config;
            }
        };

        // Theme first, so config tokens have something to lay over.
        let theme_node = doc.nodes().iter().find(|n| n.name().value() == "theme");
        let theme_name = theme_node.and_then(|n| arg_string(n, 0)).unwrap_or("dusk");
        let (theme, mut theme_problems) = Theme::load(theme_name, beside);
        config.problems.append(&mut theme_problems);
        config.theme = theme;

        let mut base_tokens = Vec::new();
        let mut light_tokens = Vec::new();
        for node in doc.nodes() {
            let name = node.name().value();
            let line = node_line(source, node);
            match name {
                "theme" => {
                    if node.entries().iter().filter(|e| e.name().is_none()).count() != 1 {
                        config.problems.push(Problem::new(
                            file,
                            line,
                            "theme needs one id or path",
                        ));
                    }
                }
                "layout" => match arg_string(node, 0).and_then(Layout::parse) {
                    Some(layout) => config.layout = Some(layout),
                    None => config.problems.push(Problem::new(
                        file,
                        line,
                        "layout needs row, grid or list",
                    )),
                },
                "appearance" => match arg_string(node, 0).and_then(Appearance::parse) {
                    Some(appearance) => config.appearance = appearance,
                    None => config.problems.push(Problem::new(
                        file,
                        line,
                        "appearance needs auto, light or dark",
                    )),
                },
                "hide" => {
                    let args = string_args(node);
                    if args.is_empty() {
                        config.problems.push(Problem::new(
                            file,
                            line,
                            "hide needs at least one action id",
                        ));
                    }
                    config.hide.extend(args);
                }
                "order" => {
                    let args = string_args(node);
                    if args.is_empty() {
                        config.problems.push(Problem::new(
                            file,
                            line,
                            "order needs at least one action id",
                        ));
                    }
                    config.order.extend(args);
                }
                "tokens" => match arg_string(node, 0) {
                    Some("light") => light_tokens.push(node),
                    Some(other) => config.problems.push(Problem::new(
                        file,
                        line,
                        format!("tokens takes no name (or \"light\"), not \"{other}\""),
                    )),
                    None => base_tokens.push(node),
                },
                "action" => match arg_string(node, 0) {
                    Some(id) => parse_action(
                        id,
                        node,
                        file,
                        source,
                        &mut config.actions,
                        &mut config.problems,
                    ),
                    None => config.problems.push(Problem::new(
                        file,
                        line,
                        "action needs an id, for example action \"shutdown\" { ... }",
                    )),
                },
                other => config.problems.push(Problem::new(
                    file,
                    line,
                    format!("unknown setting \"{other}\""),
                )),
            }
        }

        for node in base_tokens {
            config
                .theme
                .apply_config_tokens(Some(node), None, file, source, &mut config.problems);
        }
        for node in light_tokens {
            config
                .theme
                .apply_config_tokens(None, Some(node), file, source, &mut config.problems);
        }

        // Keep the last action node for a repeated id, so a later override wins.
        let mut seen = std::collections::HashSet::new();
        let mut actions = Vec::new();
        for patch in config.actions.iter().rev() {
            if seen.insert(patch.id.clone()) {
                actions.push(patch.clone());
            } else {
                config.problems.push(Problem::new(
                    file,
                    patch.line,
                    format!(
                        "action \"{}\" is set twice; the earlier one is ignored",
                        patch.id
                    ),
                ));
            }
        }
        actions.reverse();
        config.actions = actions;
        let mut seen = std::collections::HashSet::new();
        config
            .problems
            .retain(|p| seen.insert((p.file.clone(), p.line, p.message.clone())));
        config
    }
}

fn parse_action(
    id: &str,
    node: &KdlNode,
    file: &str,
    source: &str,
    actions: &mut Vec<ActionPatch>,
    problems: &mut Vec<Problem>,
) {
    let line = node_line(source, node);
    let mut patch = ActionPatch::new(id.to_owned(), line);
    if let Some(children) = node.children() {
        for child in children.nodes() {
            let field = child.name().value();
            let field_line = node_line(source, child);
            let value = child
                .entries()
                .iter()
                .find(|e| e.name().is_none())
                .map(|e| e.value());
            match field {
                "label" => match value.and_then(|v| v.as_string()) {
                    Some(v) => patch.label = Some(v.to_owned()),
                    None => problems.push(Problem::new(
                        file,
                        field_line,
                        "label needs a quoted string",
                    )),
                },
                "icon" => match value.and_then(|v| v.as_string()) {
                    Some(v) => patch.icon = Some(v.to_owned()),
                    None => problems.push(Problem::new(
                        file,
                        field_line,
                        "icon needs a name or a path",
                    )),
                },
                "run" => match value.and_then(|v| v.as_string()) {
                    Some(v) => patch.run = Some(v.to_owned()),
                    None => {
                        problems.push(Problem::new(file, field_line, "run needs a quoted command"))
                    }
                },
                "key" => match value {
                    Some(v) if v.as_string().is_some() => {
                        let raw = v.as_string().unwrap_or_default();
                        let mut chars = raw.chars();
                        match (chars.next(), chars.next()) {
                            (Some(c), None) if !c.is_control() => {
                                patch.key = Some(Some(c.to_ascii_lowercase()))
                            }
                            (None, _) => patch.key = Some(None),
                            _ => problems.push(Problem::new(
                                file,
                                field_line,
                                "key must be one character",
                            )),
                        }
                    }
                    Some(v) if v.as_bool() == Some(false) || v.to_string() == "#null" => {
                        patch.key = Some(None)
                    }
                    _ => problems.push(Problem::new(
                        file,
                        field_line,
                        "key needs one quoted character",
                    )),
                },
                "confirm" => match value.and_then(|v| v.as_bool()) {
                    Some(v) => patch.confirm = Some(v),
                    None => problems.push(Problem::new(
                        file,
                        field_line,
                        "confirm needs #true or #false",
                    )),
                },
                other => problems.push(Problem::new(
                    file,
                    field_line,
                    format!("unknown action field \"{other}\""),
                )),
            }
        }
    }
    actions.push(patch);
}

fn string_args(node: &KdlNode) -> Vec<String> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .filter_map(|e| e.value().as_string().map(str::to_owned))
        .collect()
}

fn arg_string(node: &KdlNode, index: usize) -> Option<&str> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .nth(index)?
        .value()
        .as_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Layout;

    #[test]
    fn the_section_five_example_parses() {
        let source = r##"
theme "dusk"
layout "row"
tokens { accent "#d38be0"; radius 28 }
tokens "light" { dim "white/40" }
hide "hibernate"
order "lock" "logout" "suspend" "reboot" "shutdown"
action "logout" { run "hyprshutdown"; key "e" }
action "firmware" {
    label "Firmware"
    icon "~/.config/dusk/icons/chip.svg"
    run "systemctl reboot --firmware-setup"
    key "f"
    confirm #true
}
"##;
        let config = Config::from_source(source, "config.kdl", None);
        assert!(config.problems.is_empty(), "{:?}", config.problems);
        assert_eq!(config.layout, Some(Layout::Row));
        assert_eq!(config.hide, vec!["hibernate"]);
        assert_eq!(config.order.len(), 5);
        let firmware = config.actions.iter().find(|a| a.id == "firmware").unwrap();
        assert_eq!(firmware.key, Some(Some('f')));
        assert_eq!(firmware.confirm, Some(true));
        assert_eq!(config.theme.tokens(true).radius, 28.0);
        assert!((config.theme.tokens(false).dim.a - 0.4).abs() < 1e-6);
    }

    #[test]
    fn base_tokens_apply_before_light_tokens_whatever_the_file_order() {
        let source = "tokens \"light\" { radius 30 }\ntokens { radius 10 }\n";
        let config = Config::from_source(source, "config.kdl", None);
        assert_eq!(config.theme.tokens(true).radius, 10.0);
        assert_eq!(config.theme.tokens(false).radius, 30.0);
    }

    #[test]
    fn a_bad_token_keeps_the_actions_that_parse() {
        let source =
            "tokens { radus 12 }\naction \"firmware\" { label \"Firmware\"; run \"true\" }\n";
        let config = Config::from_source(source, "config.kdl", None);
        assert!(
            config
                .problems
                .iter()
                .any(|p| p.line == 1 && p.message.contains("radus"))
        );
        assert!(config.actions.iter().any(|a| a.id == "firmware"));
    }

    #[test]
    fn problems_carry_the_line_of_the_bad_field() {
        let source = "theme \"dusk\"\n\naction \"firmware\" {\n    label \"Firmware\"\n    conirm #true\n}\n";
        let config = Config::from_source(source, "config.kdl", None);
        let p = config
            .problems
            .iter()
            .find(|p| p.message.contains("conirm"))
            .unwrap();
        assert_eq!(p.line, 5);
    }

    #[test]
    fn duplicate_keys_are_left_for_resolution_to_report() {
        let source = "action \"a\" { label \"A\"; run \"true\"; key \"x\" }\naction \"b\" { label \"B\"; run \"true\"; key \"X\" }\n";
        let config = Config::from_source(source, "config.kdl", None);
        assert!(config.problems.is_empty());
        let keys: Vec<_> = config
            .actions
            .iter()
            .filter_map(|a| a.key.flatten())
            .collect();
        assert_eq!(keys, vec!['x', 'x']);
    }
}
