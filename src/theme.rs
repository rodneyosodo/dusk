//! Tokens and themes. A theme is looks only: layout, `compact`/`mono`, and tokens.
//! Actions and behaviour live in `config.kdl`, so a downloaded theme can never run commands.

use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};
use rune::Color;
use rune::color::hex;

use crate::problem::{Problem, node_line};

pub const DUSK: &str = include_str!("../themes/dusk.kdl");
pub const HALO: &str = include_str!("../themes/halo.kdl");
pub const MONO: &str = include_str!("../themes/mono.kdl");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Row,
    Grid,
    List,
}

impl Layout {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "row" => Some(Self::Row),
            "grid" => Some(Self::Grid),
            "list" => Some(Self::List),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Auto,
    Light,
    Dark,
}

impl Appearance {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// The 30-odd look tokens, documented in `docs/THEMING.md`.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    pub accent: Color,
    pub dim: Color,
    pub backdrop_top: Color,
    pub backdrop_bottom: Color,
    pub card: Color,
    pub card_border: Color,
    pub shadow: Color,
    pub icon: Color,
    pub text: Color,
    pub text_dim: Color,
    pub selected_text: Color,
    pub warning: Color,
    pub lock: Color,
    pub suspend: Color,
    pub logout: Color,
    pub reboot: Color,
    pub shutdown: Color,
    /// Two soft glows in the backdrop: cool top-left, warm bottom-right.
    pub bloom_a: Color,
    pub bloom_b: Color,

    pub radius: f32,
    pub card_w: f32,
    pub card_h: f32,
    pub gap: f32,
    pub icon_size: f32,
    pub label_size: f32,
    pub label_weight: f32,
    pub key_size: f32,
    pub padding: f32,
    /// The tinted disc behind each icon; 0 draws the icon bare.
    pub orb_size: f32,
    pub enter_ms: f32,
    pub exit_ms: f32,
    pub stagger_ms: f32,
    pub spring_bounce: f32,

    /// An icon-only pill at screen centre.
    pub compact: bool,
    /// Use the system monospace family for labels and keys.
    pub mono: bool,
    /// The time, date and a short line above the cards.
    pub header: bool,
}

impl Default for Tokens {
    fn default() -> Self {
        Self::dusk_dark()
    }
}

impl Tokens {
    pub fn dusk_dark() -> Self {
        Self {
            accent: hex("#c79bff").expect("valid accent"),
            dim: hex("#07060c").expect("valid").alpha(0.62),
            backdrop_top: hex("#1a1433").expect("valid").alpha(0.70),
            backdrop_bottom: hex("#2b1220").expect("valid").alpha(0.66),
            card: hex("#0b0a12").expect("valid").alpha(0.82),
            card_border: Color::WHITE.alpha(0.12),
            shadow: Color::BLACK.alpha(0.55),
            icon: Color::WHITE.alpha(0.96),
            text: hex("#f6f2fb").expect("valid"),
            text_dim: Color::WHITE.alpha(0.58),
            selected_text: hex("#0d0a12").expect("valid"),
            warning: hex("#ff8b6b").expect("valid"),
            lock: hex("#8ec8ff").expect("valid"),
            suspend: hex("#8be3d2").expect("valid"),
            logout: hex("#d99af0").expect("valid"),
            reboot: hex("#ffc46b").expect("valid"),
            shutdown: hex("#ff7b6b").expect("valid"),
            bloom_a: hex("#5b4bff").expect("valid").alpha(0.30),
            bloom_b: hex("#ff6a4d").expect("valid").alpha(0.22),
            radius: 28.0,
            card_w: 150.0,
            card_h: 170.0,
            gap: 18.0,
            icon_size: 26.0,
            label_size: 14.5,
            label_weight: 560.0,
            key_size: 10.5,
            padding: 16.0,
            orb_size: 62.0,
            enter_ms: 260.0,
            exit_ms: 120.0,
            stagger_ms: 32.0,
            spring_bounce: 0.06,
            compact: false,
            mono: false,
            header: true,
        }
    }

    pub fn dusk_light() -> Self {
        let mut t = Self::dusk_dark();
        t.dim = hex("#f6f3fa").expect("valid").alpha(0.62);
        t.backdrop_top = hex("#ece6fb").expect("valid").alpha(0.72);
        t.backdrop_bottom = hex("#fbeae4").expect("valid").alpha(0.66);
        t.card = Color::WHITE.alpha(0.82);
        t.card_border = Color::BLACK.alpha(0.09);
        t.shadow = hex("#2a1f45").expect("valid").alpha(0.16);
        t.bloom_a = hex("#8f7bff").expect("valid").alpha(0.28);
        t.bloom_b = hex("#ff9a7a").expect("valid").alpha(0.24);
        t.icon = hex("#211a2a").expect("valid");
        t.text = hex("#211a2a").expect("valid");
        t.text_dim = Color::BLACK.alpha(0.78);
        t.selected_text = Color::WHITE;
        t.warning = hex("#c2412d").expect("valid");
        t.lock = hex("#2c6fb5").expect("valid");
        t.suspend = hex("#177f72").expect("valid");
        t.logout = hex("#8a3aa8").expect("valid");
        t.reboot = hex("#a55d00").expect("valid");
        t.shutdown = hex("#b02a22").expect("valid");
        t
    }

    pub fn action_hue(&self, id: &str) -> Color {
        match id {
            "lock" => self.lock,
            "suspend" | "hibernate" => self.suspend,
            "logout" => self.logout,
            "reboot" => self.reboot,
            "shutdown" => self.shutdown,
            _ => self.accent,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub id: String,
    pub layout: Layout,
    pub dark: Tokens,
    pub light: Tokens,
    /// The theme file a preview should watch; `None` for a built-in.
    pub source: Option<PathBuf>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            id: "dusk".into(),
            layout: Layout::Row,
            dark: Tokens::dusk_dark(),
            light: Tokens::dusk_light(),
            source: None,
        }
    }
}

impl Theme {
    pub fn tokens(&self, dark: bool) -> &Tokens {
        if dark { &self.dark } else { &self.light }
    }

    /// A built-in theme by id, without a search path.
    pub fn builtin(id: &str) -> Option<(Theme, Vec<Problem>)> {
        let (source, label) = match id {
            "dusk" => (DUSK, "themes/dusk.kdl"),
            "halo" => (HALO, "themes/halo.kdl"),
            "mono" => (MONO, "themes/mono.kdl"),
            _ => return None,
        };
        let mut problems = Vec::new();
        let mut theme = Theme {
            id: id.into(),
            ..Theme::default()
        };
        apply_theme_source(&mut theme, source, label, &mut problems);
        Some((theme, problems))
    }

    /// A built-in id, a path, or `themes/<id>.kdl` beside the config.
    pub fn load(name: &str, beside: Option<&Path>) -> (Theme, Vec<Problem>) {
        let mut problems = Vec::new();
        let mut theme = if let Some((theme, p)) = Theme::builtin(name) {
            problems.extend(p);
            theme
        } else if let Some((source, source_path)) = read_candidate(name, beside) {
            let mut theme = Theme {
                id: name.into(),
                source: Some(source_path.clone()),
                ..Theme::default()
            };
            apply_theme_source(
                &mut theme,
                &source,
                &source_path.display().to_string(),
                &mut problems,
            );
            theme.source = Some(source_path);
            theme
        } else {
            problems.push(Problem::new(
                "<theme>",
                1,
                format!("theme \"{name}\" not found; using dusk"),
            ));
            Theme::default()
        };
        theme.id = name.into();
        (theme, problems)
    }

    /// Applies a config's own `tokens` on top of the theme. `base` goes to both appearances,
    /// `light` only to the light one.
    pub fn apply_config_tokens(
        &mut self,
        base: Option<&KdlNode>,
        light: Option<&KdlNode>,
        file: &str,
        source: &str,
        problems: &mut Vec<Problem>,
    ) {
        if let Some(node) = base {
            apply_token_node(&mut self.dark, node, file, source, problems);
            apply_token_node(&mut self.light, node, file, source, problems);
        }
        if let Some(node) = light {
            apply_token_node(&mut self.light, node, file, source, problems);
        }
    }
}

/// Where a theme selection would live on disk, even if the file is missing. Built-ins return
/// `None` because they are embedded; preview uses this to keep watching a theme that is being
/// deleted/recreated.
pub fn theme_candidate_path(name: &str, beside: Option<&Path>) -> Option<PathBuf> {
    if Theme::builtin(name).is_some() {
        return None;
    }
    let path = if let Some(rest) = name.strip_prefix("~/") {
        match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => PathBuf::from(name),
        }
    } else {
        PathBuf::from(name)
    };
    if path.is_absolute() || name.contains('/') || name.ends_with(".kdl") {
        return Some(path);
    }
    if let Some(dir) = beside {
        return Some(dir.join("themes").join(format!("{name}.kdl")));
    }
    config_dir().map(|dir| dir.join("themes").join(format!("{name}.kdl")))
}

fn read_candidate(name: &str, beside: Option<&Path>) -> Option<(String, PathBuf)> {
    let expanded = if let Some(rest) = name.strip_prefix("~/") {
        std::env::var_os("HOME").map_or_else(
            || PathBuf::from(name),
            |home| PathBuf::from(home).join(rest),
        )
    } else {
        PathBuf::from(name)
    };
    let path = expanded.as_path();
    if path.is_file() {
        return std::fs::read_to_string(path)
            .ok()
            .map(|s| (s, path.to_path_buf()));
    }
    if let Some(dir) = beside {
        let direct = dir.join(name);
        if direct.is_file() {
            return std::fs::read_to_string(&direct).ok().map(|s| (s, direct));
        }
        let p = dir.join("themes").join(format!("{name}.kdl"));
        if p.is_file() {
            return std::fs::read_to_string(&p).ok().map(|s| (s, p));
        }
    }
    if !name.contains('/')
        && let Some(config) = config_dir()
    {
        let p = config.join("themes").join(format!("{name}.kdl"));
        if p.is_file() {
            return std::fs::read_to_string(&p).ok().map(|s| (s, p));
        }
    }
    None
}

pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|p| p.join("dusk"))
}

fn apply_theme_source(theme: &mut Theme, source: &str, file: &str, problems: &mut Vec<Problem>) {
    let doc: KdlDocument = match source.parse() {
        Ok(doc) => doc,
        Err(err) => {
            let line = err
                .diagnostics
                .first()
                .map_or(1, |d| crate::problem::line_at(source, d.span.offset()));
            problems.push(Problem::new(file, line, "this theme is not valid KDL"));
            return;
        }
    };
    // A theme may extend a built-in base before applying its own looks.
    let mut base = theme.clone();
    for node in doc.nodes() {
        if node.name().value() == "extends" {
            let Some(name) = arg_string(node, 0) else {
                problems.push(Problem::new(
                    file,
                    node_line(source, node),
                    "extends needs a theme id",
                ));
                continue;
            };
            match Theme::builtin(name) {
                Some((b, p)) => {
                    problems.extend(p);
                    base = b;
                }
                None => problems.push(Problem::new(
                    file,
                    node_line(source, node),
                    format!("unknown base theme \"{name}\""),
                )),
            }
        }
    }
    let mut merged = base;
    for node in doc.nodes() {
        match node.name().value() {
            "extends" => {}
            "layout" => {
                let Some(value) = arg_string(node, 0) else {
                    problems.push(Problem::new(
                        file,
                        node_line(source, node),
                        "layout needs row, grid or list",
                    ));
                    continue;
                };
                match Layout::parse(value) {
                    Some(layout) => merged.layout = layout,
                    None => problems.push(Problem::new(
                        file,
                        node_line(source, node),
                        format!("unknown layout \"{value}\"; expected row, grid or list"),
                    )),
                }
            }
            "tokens" => match arg_string(node, 0) {
                Some("light") => apply_token_node(&mut merged.light, node, file, source, problems),
                Some(other) => problems.push(Problem::new(
                    file,
                    node_line(source, node),
                    format!("unknown token set \"{other}\"; only \"light\" is valid"),
                )),
                None => {
                    apply_token_node(&mut merged.dark, node, file, source, problems);
                    apply_token_node(&mut merged.light, node, file, source, problems);
                }
            },
            other => problems.push(Problem::new(
                file,
                node_line(source, node),
                format!("themes are looks only: \"{other}\" belongs in config.kdl and is ignored"),
            )),
        }
    }
    merged.id = theme.id.clone();
    *theme = merged;
}

fn arg_string(node: &KdlNode, index: usize) -> Option<&str> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .nth(index)?
        .value()
        .as_string()
}

fn value_as_f32(value: &KdlValue) -> Option<f32> {
    value
        .as_float()
        .map(|v| v as f32)
        .or_else(|| value.as_integer().map(|v| v as f32))
}

fn color_of_token<'a>(tokens: &'a Tokens, name: &str) -> Option<&'a Color> {
    Some(match name {
        "accent" => &tokens.accent,
        "dim" => &tokens.dim,
        "backdrop-top" | "backdrop_top" => &tokens.backdrop_top,
        "backdrop-bottom" | "backdrop_bottom" => &tokens.backdrop_bottom,
        "card" => &tokens.card,
        "card-border" | "card_border" => &tokens.card_border,
        "shadow" => &tokens.shadow,
        "icon" => &tokens.icon,
        "text" => &tokens.text,
        "text-dim" | "text_dim" => &tokens.text_dim,
        "selected-text" | "selected_text" => &tokens.selected_text,
        "warning" => &tokens.warning,
        "lock" => &tokens.lock,
        "suspend" => &tokens.suspend,
        "logout" => &tokens.logout,
        "reboot" => &tokens.reboot,
        "shutdown" => &tokens.shutdown,
        "bloom-a" | "bloom_a" => &tokens.bloom_a,
        "bloom-b" | "bloom_b" => &tokens.bloom_b,
        _ => return None,
    })
}

fn set_color_token(tokens: &mut Tokens, name: &str, color: Color) -> Result<(), String> {
    match name {
        "accent" => tokens.accent = color,
        "dim" => tokens.dim = color,
        "backdrop-top" | "backdrop_top" => tokens.backdrop_top = color,
        "backdrop-bottom" | "backdrop_bottom" => tokens.backdrop_bottom = color,
        "card" => tokens.card = color,
        "card-border" | "card_border" => tokens.card_border = color,
        "shadow" => tokens.shadow = color,
        "icon" => tokens.icon = color,
        "text" => tokens.text = color,
        "text-dim" | "text_dim" => tokens.text_dim = color,
        "selected-text" | "selected_text" => tokens.selected_text = color,
        "warning" => tokens.warning = color,
        "lock" => tokens.lock = color,
        "suspend" => tokens.suspend = color,
        "logout" => tokens.logout = color,
        "reboot" => tokens.reboot = color,
        "shutdown" => tokens.shutdown = color,
        "bloom-a" | "bloom_a" => tokens.bloom_a = color,
        "bloom-b" | "bloom_b" => tokens.bloom_b = color,
        other => return Err(format!("unknown colour token \"{other}\"")),
    }
    Ok(())
}

/// Applies one `tokens` block. Colour references may point anywhere in the block.
fn apply_token_node(
    tokens: &mut Tokens,
    node: &KdlNode,
    file: &str,
    source: &str,
    problems: &mut Vec<Problem>,
) {
    let Some(children) = node.children() else {
        problems.push(Problem::new(
            file,
            node_line(source, node),
            "tokens needs a block, for example tokens { radius 28 }",
        ));
        return;
    };
    let mut colors: Vec<(String, String, usize)> = Vec::new();
    for child in children.nodes() {
        let name = child.name().value();
        let Some(value) = child
            .entries()
            .iter()
            .find(|e| e.name().is_none())
            .map(|e| e.value())
        else {
            problems.push(Problem::new(
                file,
                node_line(source, child),
                format!("token \"{name}\" needs a value"),
            ));
            continue;
        };
        let line = node_line(source, child);
        match name {
            "radius" => set_number(&mut tokens.radius, value, file, line, name, problems),
            "card-width" | "card_width" => {
                set_number(&mut tokens.card_w, value, file, line, name, problems)
            }
            "card-height" | "card_height" => {
                set_number(&mut tokens.card_h, value, file, line, name, problems)
            }
            "gap" => set_number(&mut tokens.gap, value, file, line, name, problems),
            "icon-size" | "icon_size" => {
                set_number(&mut tokens.icon_size, value, file, line, name, problems)
            }
            "label-size" | "label_size" => {
                set_number(&mut tokens.label_size, value, file, line, name, problems)
            }
            "label-weight" | "label_weight" => {
                set_number(&mut tokens.label_weight, value, file, line, name, problems)
            }
            "key-size" | "key_size" => {
                set_number(&mut tokens.key_size, value, file, line, name, problems)
            }
            "padding" => set_number(&mut tokens.padding, value, file, line, name, problems),
            "orb-size" | "orb_size" => {
                set_number(&mut tokens.orb_size, value, file, line, name, problems)
            }
            "header" => match value.as_bool() {
                Some(v) => tokens.header = v,
                None => problems.push(Problem::new(file, line, "header needs #true or #false")),
            },
            "enter-ms" | "enter_ms" => {
                set_number(&mut tokens.enter_ms, value, file, line, name, problems)
            }
            "exit-ms" | "exit_ms" => {
                set_number(&mut tokens.exit_ms, value, file, line, name, problems)
            }
            "stagger-ms" | "stagger_ms" => {
                set_number(&mut tokens.stagger_ms, value, file, line, name, problems)
            }
            "spring-bounce" | "spring_bounce" => {
                set_number(&mut tokens.spring_bounce, value, file, line, name, problems)
            }
            "compact" => match value.as_bool() {
                Some(v) => tokens.compact = v,
                None => problems.push(Problem::new(file, line, "compact needs #true or #false")),
            },
            "mono" => match value.as_bool() {
                Some(v) => tokens.mono = v,
                None => problems.push(Problem::new(file, line, "mono needs #true or #false")),
            },
            _ if color_of_token(tokens, name).is_some() => match value.as_string() {
                Some(raw) => colors.push((name.to_owned(), raw.to_owned(), line)),
                None => problems.push(Problem::new(
                    file,
                    line,
                    format!("colour token \"{name}\" needs a quoted value"),
                )),
            },
            other => problems.push(Problem::new(
                file,
                line,
                format!("unknown token \"{other}\""),
            )),
        }
    }

    // Resolve in passes so `card "accent/20"` may name a token defined below it.
    for _ in 0..8 {
        let before = colors.len();
        let mut still = Vec::new();
        for (name, raw, line) in colors {
            let lookup = |n: &str| color_of_token(tokens, n).copied();
            match parse_color(&raw, &lookup) {
                Ok(color) => set_color_token(tokens, &name, color).expect("known token"),
                Err(err) => still.push((name, raw, line, err)),
            }
        }
        if still.is_empty() {
            return;
        }
        colors = still
            .iter()
            .map(|(n, r, l, _)| (n.clone(), r.clone(), *l))
            .collect();
        if colors.len() == before {
            for (name, raw, line, err) in still {
                problems.push(Problem::new(
                    file,
                    line,
                    format!("token \"{name}\": {err} in \"{raw}\""),
                ));
            }
            return;
        }
    }
}

fn set_number(
    slot: &mut f32,
    value: &KdlValue,
    file: &str,
    line: usize,
    name: &str,
    problems: &mut Vec<Problem>,
) {
    match value_as_f32(value) {
        Some(v) if v.is_finite() => *slot = v,
        Some(_) => problems.push(Problem::new(
            file,
            line,
            format!("token \"{name}\" must be a finite number"),
        )),
        None => problems.push(Problem::new(
            file,
            line,
            format!("token \"{name}\" needs a number"),
        )),
    }
}

/// A Hokum-style colour: `"#d38be0"`, `"white/40"`, `"accent/25"`, `"oklch(0.7 0.13 300 / 40)"`.
pub fn parse_color(raw: &str, lookup: &dyn Fn(&str) -> Option<Color>) -> Result<Color, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("empty colour".into());
    }
    if let Some(inner) = raw.strip_prefix("oklch(").and_then(|s| s.strip_suffix(')')) {
        return parse_oklch(inner);
    }
    if let Some(inner) = raw.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        return parse_rgb(inner);
    }
    let (base, alpha) = split_alpha(raw);
    let mut color = match base {
        "white" => Color::WHITE,
        "black" => Color::BLACK,
        "transparent" | "none" => Color::TRANSPARENT,
        other => {
            if let Some(c) = hex(other) {
                c
            } else if let Some(c) = lookup(other) {
                c
            } else {
                return Err(format!("unknown colour \"{other}\""));
            }
        }
    };
    if let Some(a) = alpha {
        color.a = parse_alpha(a)?;
    }
    Ok(color)
}

fn split_alpha(raw: &str) -> (&str, Option<&str>) {
    // A slash after the closing parenthesis (if any) is the alpha.
    if let Some(i) = raw.rfind('/') {
        (&raw[..i], Some(&raw[i + 1..]))
    } else {
        (raw, None)
    }
}

fn parse_alpha(a: &str) -> Result<f32, String> {
    let a = a.trim();
    let value: f32 = a
        .parse()
        .map_err(|_| format!("alpha \"{a}\" is not a number"))?;
    let alpha = if a.contains('.') {
        value
    } else {
        value / 100.0
    };
    if !(0.0..=1.0).contains(&alpha) {
        return Err(format!("alpha \"{a}\" must be 0..100 or 0..1"));
    }
    Ok(alpha)
}

fn parse_rgb(inner: &str) -> Result<Color, String> {
    let parts: Vec<&str> = inner
        .split(&[',', ' '][..])
        .filter(|s| !s.trim().is_empty())
        .collect();
    if parts.len() < 3 {
        return Err("rgb() needs three channels".into());
    }
    let ch = |s: &str| -> Result<f32, String> {
        let s = s.trim();
        if let Some(p) = s.strip_suffix('%') {
            p.parse::<f32>()
                .map(|v| v / 100.0)
                .map_err(|_| format!("bad channel \"{s}\""))
        } else {
            s.parse::<f32>()
                .map(|v| v / 255.0)
                .map_err(|_| format!("bad channel \"{s}\""))
        }
    };
    Ok(Color {
        r: ch(parts[0])?,
        g: ch(parts[1])?,
        b: ch(parts[2])?,
        a: 1.0,
    })
}

fn parse_oklch(inner: &str) -> Result<Color, String> {
    let (body, alpha) = inner
        .split_once('/')
        .map_or((inner, None), |(b, a)| (b, Some(a)));
    let parts: Vec<&str> = body.split_whitespace().filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return Err("oklch() needs lightness, chroma and hue".into());
    }
    let l: f32 = parts[0].parse().map_err(|_| "bad oklch lightness")?;
    let c: f32 = parts[1].parse().map_err(|_| "bad oklch chroma")?;
    let h: f32 = parts[2].parse().map_err(|_| "bad oklch hue")?;
    let a = alpha.map_or(Ok(1.0), parse_alpha)?;
    Ok(rune::oklcha(l, c, h, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_lookup(_: &str) -> Option<Color> {
        None
    }

    #[test]
    fn parses_hex_named_and_alpha_colours() {
        assert_eq!(parse_color("#ffffff", &no_lookup).unwrap(), Color::WHITE);
        assert_eq!(parse_color("white/40", &no_lookup).unwrap().a, 0.4);
        assert_eq!(parse_color("black/12", &no_lookup).unwrap().a, 0.12);
        assert!((parse_color("white/0.4", &no_lookup).unwrap().a - 0.4).abs() < 1e-6);
        assert!(parse_color("white/140", &no_lookup).is_err());
    }

    #[test]
    fn every_builtin_theme_parses_without_problems() {
        for id in ["dusk", "halo", "mono"] {
            let (theme, problems) = Theme::builtin(id).expect("built-in theme");
            assert!(problems.is_empty(), "{id}: {problems:?}");
            assert!(theme.dark.card_w > 0.0);
            assert!(theme.light.card_w > 0.0);
        }
    }

    #[test]
    fn a_theme_that_sets_actions_is_reported_and_ignored() {
        let mut theme = Theme::default();
        let mut problems = Vec::new();
        apply_theme_source(
            &mut theme,
            "layout \"list\"\naction \"shutdown\" { run \"poweroff\" }\n",
            "t.kdl",
            &mut problems,
        );
        assert_eq!(theme.layout, Layout::List);
        assert!(
            problems.iter().any(|p| p.message.contains("looks only")),
            "{problems:?}"
        );
    }

    #[test]
    fn colour_tokens_can_reference_other_tokens_in_any_order() {
        let mut tokens = Tokens::dusk_dark();
        let mut problems = Vec::new();
        let doc: KdlDocument = "tokens {\n  card \"accent/20\"\n  accent \"#ff0000\"\n}"
            .parse()
            .unwrap();
        apply_token_node(
            &mut tokens,
            doc.get("tokens").unwrap(),
            "c.kdl",
            "tokens {\n  card \"accent/20\"\n  accent \"#ff0000\"\n}",
            &mut problems,
        );
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(tokens.accent, hex("#ff0000").unwrap());
        assert!((tokens.card.a - 0.2).abs() < 1e-6);
    }
    fn over(top: Color, bottom: Color) -> Color {
        let a = top.a + bottom.a * (1.0 - top.a);
        if a <= 0.0 {
            return Color::TRANSPARENT;
        }
        let mix = |t: f32, b: f32| (t * top.a + b * bottom.a * (1.0 - top.a)) / a;
        Color {
            r: mix(top.r, bottom.r),
            g: mix(top.g, bottom.g),
            b: mix(top.b, bottom.b),
            a,
        }
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (x, y) = (a.luminance(), b.luminance());
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn text_holds_contrast_on_the_worst_wallpaper() {
        for id in ["dusk", "halo", "mono"] {
            let (theme, _) = Theme::builtin(id).expect("built-in theme");
            for (name, t) in [("dark", &theme.dark), ("light", &theme.light)] {
                for wallpaper in [Color::WHITE, Color::BLACK] {
                    let bg = over(t.card, over(t.dim, wallpaper));
                    // Unselected labels are drawn at 0.90, icons and hints at full token
                    // alpha, so test the ink as rendered rather than the bare token.
                    let checks = [
                        ("selected label", over(t.text, bg), 4.5),
                        ("label", over(t.text.alpha(0.90), bg), 4.5),
                        ("dim text", over(t.text_dim, bg), 3.0),
                        ("hint", over(t.text_dim, bg), 3.0),
                        ("icon", over(t.icon, bg), 3.0),
                    ];
                    for (what, ink, want) in checks {
                        let got = contrast(ink, bg);
                        assert!(
                            got >= want,
                            "{id} {name} {what} contrast {got:.2} < {want} on {}",
                            wallpaper.hex()
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn a_file_theme_records_the_source_path() {
        let dir = std::env::temp_dir().join(format!("dusk-theme-test-{}", std::process::id()));
        let themes = dir.join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(themes.join("foo.kdl"), "extends \"dusk\"\n").unwrap();
        let (theme, problems) = Theme::load("foo", Some(&dir));
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            theme.source.as_deref(),
            Some(themes.join("foo.kdl").as_path())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
