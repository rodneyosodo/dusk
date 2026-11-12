//! One action: the glass card with its glowing icon orb, the list row used by `mono`, keycaps,
//! and the armed countdown ring.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use rune::{
    Color, Cursor, El, Family, Icon, LineCap, Shadow, Shape, TextStyle, canvas, col, el, icon, row,
    text,
};

use crate::actions::{Action, Armed, CONFIRM_WINDOW, IconRef, armed_label};
use crate::model::MenuState;
use crate::theme::Tokens;

/// How far the selected card lifts, in points.
pub const LIFT: f32 = 4.0;

/// The hue a card glows in: the warning colour while armed, otherwise the action's own.
pub fn hue(t: &Tokens, s: &MenuState, action: &Action) -> Color {
    if s.armed.as_ref().is_some_and(|a| a.id == action.id) {
        t.warning
    } else {
        t.action_hue(action.hue_id())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn card(
    s: &MenuState,
    t: &Tokens,
    index: usize,
    action: &Action,
    exit: f32,
    reduced: bool,
    compact: bool,
) -> El {
    let armed = s.armed.as_ref().filter(|a| a.id == action.id);
    let (opacity, offset, blur) = entry(s, index, t, reduced, exit);
    let selected = s.selected == index;
    let chosen = s.chosen.as_deref() == Some(action.id.as_str()) && s.exit.is_some();
    let scale = if chosen {
        1.0 + 0.04 * (1.0 - exit)
    } else {
        1.0
    };

    let mut cell = col()
        .key(format!("dusk/card/{}", action.id))
        .w_full()
        .h_full()
        .center()
        .padding(if compact { 0.0 } else { t.padding });

    if compact {
        let ink = if selected { t.icon } else { t.icon.alpha(0.66) };
        cell = cell.child(icon_for(action, t.icon_size, ink));
        if let Some(armed) = armed {
            let d = t.card_w.min(t.card_h) + 8.0;
            cell = cell.child(ring(t, armed, s.now, d).absolute(-4.0, -4.0));
        }
    } else {
        cell = cell.gap(12.0).child(orb(t, s, action, selected, armed));
        let label = if armed.is_some() {
            armed_label(action)
        } else {
            action.label.clone()
        };
        let color = if armed.is_some() {
            t.warning
        } else if selected {
            t.text
        } else {
            t.text.alpha(0.74)
        };
        let family = if t.mono {
            Family::SystemMono
        } else {
            Family::Sans
        };
        let label_h = t.label_size * 1.3 * 2.0;
        cell = cell.child(
            col().h(label_h).w_full().center().child(
                text(label)
                    .font(
                        TextStyle::new(t.label_size, t.label_weight)
                            .family(family)
                            .tracking(-0.1),
                    )
                    .color(color)
                    .wrap()
                    .max_lines(2)
                    .align(rune::Align::Center),
            ),
        );
        if (s.keyboard || t.mono)
            && let Some(key) = action.key
        {
            cell = cell.child(keycap(t, &key.to_ascii_uppercase().to_string()));
        } else {
            // Keep every card's layout the same height whether keys show or not.
            cell = cell.child(el().h(18.0));
        }
        if let Some(armed) = armed.filter(|_| t.orb_size <= 0.0) {
            cell = cell.child(frame_countdown(t, armed, s.now));
        }
        cell = glass(cell, t, selected)
            .radius(t.radius)
            .continuous_corners();
    }

    let lift = if selected && !compact && !reduced {
        -LIFT
    } else {
        0.0
    };
    let mut cell = cell
        .cursor(Cursor::Pointer)
        .on_hover(move |s: &mut MenuState| s.selected = index)
        .on_press_tx({
            let action = action.clone();
            move |s: &mut MenuState, tx: &mut rune::Tx| s.press(action.clone(), tx)
        })
        .opacity(opacity)
        .scale(scale);
    if !reduced {
        cell = cell.offset(0.0, offset + lift).blur(blur);
    }
    cell
}

/// The resting glass of an unselected card. The selected card is clear: the highlight that
/// springs between cards is drawn under it.
fn glass(cell: El, t: &Tokens, selected: bool) -> El {
    if selected {
        return cell;
    }
    let edge = t.card_border;
    cell.fill(t.card)
        .gradient(edge.alpha(edge.a * 0.45), Color::TRANSPARENT)
        .border_gradient(
            1.0,
            edge.alpha((edge.a * 1.6).min(1.0)),
            edge.alpha(edge.a * 0.35),
        )
        .box_shadow(Shadow {
            y: 2.0,
            blur: 6.0,
            color: t.shadow.alpha(t.shadow.a * 0.4),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 16.0,
            blur: 36.0,
            color: t.shadow.alpha(t.shadow.a * 0.7),
            ..Shadow::default()
        })
}

/// The tinted disc behind an icon, glowing in the action's hue when selected.
fn orb(t: &Tokens, s: &MenuState, action: &Action, selected: bool, armed: Option<&Armed>) -> El {
    let hue = hue(t, s, action);
    let ink = if selected || armed.is_some() {
        t.icon
    } else {
        t.icon.alpha(0.78)
    };
    if t.orb_size <= 0.0 {
        return icon_for(action, t.icon_size, ink);
    }
    let d = t.orb_size;
    let strong = selected || armed.is_some();
    let mut orb = el()
        .key(format!("dusk/orb/{}", action.id))
        .size(d, d)
        .radius(d / 2.0)
        .center()
        .fill(hue.alpha(if strong { 0.22 } else { 0.09 }))
        .radial_gradient(
            [0.5, 0.25],
            d * 0.75,
            [
                (0.0, hue.alpha(if strong { 0.42 } else { 0.16 })),
                (1.0, Color::TRANSPARENT),
            ],
        )
        .border_gradient(
            1.0,
            hue.alpha(if strong { 0.75 } else { 0.30 }),
            hue.alpha(if strong { 0.20 } else { 0.06 }),
        )
        .child(icon_for(action, t.icon_size, ink));
    if strong {
        orb = orb.box_shadow(Shadow {
            blur: 34.0,
            color: hue.alpha(0.45),
            ..Shadow::default()
        });
    }
    if let Some(armed) = armed {
        orb = orb.child(ring(t, armed, s.now, d + 12.0).absolute(-6.0, -6.0));
    }
    orb
}

/// A circle that empties clockwise over the confirmation window.
fn ring(t: &Tokens, armed: &Armed, now: f64, d: f32) -> El {
    let remaining = ((armed.deadline - now) / CONFIRM_WINDOW).clamp(0.0, 1.0) as f32;
    let (c, r) = (d / 2.0, d / 2.0 - 1.5);
    let points: Vec<[f32; 2]> = (0..=96)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 96.0 * std::f32::consts::TAU;
            [c + r * a.cos(), c + r * a.sin()]
        })
        .collect();
    let track = Shape::new().stroke_with(
        points.clone(),
        false,
        2.0,
        LineCap::Round,
        t.warning.alpha(0.18),
    );
    let arc = Shape::new()
        .stroke_with(points, false, 2.5, LineCap::Round, t.warning)
        .trimmed(0.0, remaining);
    el().size(d, d)
        .child(canvas(track).size(d, d).absolute(0.0, 0.0))
        .child(canvas(arc).size(d, d).absolute(0.0, 0.0))
}

/// The countdown for a theme without orbs: a dashed frame that shortens around the card.
fn frame_countdown(t: &Tokens, armed: &Armed, now: f64) -> El {
    let remaining = ((armed.deadline - now) / CONFIRM_WINDOW).clamp(0.0, 1.0) as f32;
    let (w, h) = (t.card_w + 8.0, t.card_h + 8.0);
    let perimeter = 2.0 * (w + h);
    el().absolute(-4.0, -4.0)
        .size(w, h)
        .radius(t.radius + 4.0)
        .border(2.0, t.warning.alpha(0.85))
        .border_dash((perimeter * remaining).max(1.0), perimeter.max(2.0))
}

#[allow(clippy::too_many_arguments)]
pub fn list_row(
    s: &MenuState,
    t: &Tokens,
    index: usize,
    action: &Action,
    w: f32,
    h: f32,
    exit: f32,
    reduced: bool,
) -> El {
    let armed = s.armed.as_ref().filter(|a| a.id == action.id);
    let (opacity, offset, blur) = entry(s, index, t, reduced, exit);
    let selected = s.selected == index;
    let label = if armed.is_some() {
        armed_label(action)
    } else {
        action.label.clone()
    };
    let color = if armed.is_some() {
        t.warning
    } else if selected {
        t.text
    } else {
        t.text.alpha(0.72)
    };
    let mut r = row()
        .key(format!("dusk/card/{}", action.id))
        .w(w)
        .h(h)
        .px(t.padding)
        .center_y()
        .gap(12.0)
        .child(icon_for(
            action,
            t.icon_size.min(h - 12.0),
            if selected { t.icon } else { t.icon.alpha(0.65) },
        ))
        .child(
            text(label)
                .font(TextStyle::new(t.label_size, t.label_weight).family(Family::SystemMono))
                .color(color)
                .truncate()
                .grow(1.0),
        )
        .child(
            text(action_hint(action))
                .font(TextStyle::new(t.key_size, 500.0).family(Family::SystemMono))
                .color(t.text_dim.alpha(0.55)),
        );
    if let Some(key) = action.key {
        r = r.child(keycap(t, &key.to_ascii_uppercase().to_string()));
    }
    if !selected {
        r = r.fill(t.card).border(1.0, t.card_border);
    }
    let mut r = r
        .radius(t.radius)
        .cursor(Cursor::Pointer)
        .on_hover(move |s: &mut MenuState| s.selected = index)
        .on_press_tx({
            let action = action.clone();
            move |s: &mut MenuState, tx: &mut rune::Tx| s.press(action.clone(), tx)
        })
        .opacity(opacity);
    if !reduced {
        r = r.offset(0.0, offset).blur(blur);
    }
    r
}

fn action_hint(action: &Action) -> &'static str {
    match action.id.as_str() {
        "lock" => "lock",
        "logout" => "logout",
        "suspend" => "sleep",
        "hibernate" => "hibernate",
        "reboot" => "restart",
        "shutdown" => "power",
        _ => "",
    }
}

/// Opacity, rise and blur of card `index` during the staggered entrance and the exit.
fn entry(s: &MenuState, index: usize, t: &Tokens, reduced: bool, exit: f32) -> (f32, f32, f32) {
    let enter = if reduced {
        0.15
    } else {
        f64::from((t.enter_ms / 1000.0).max(0.01))
    };
    let start = s.start.unwrap_or(s.now);
    let stagger = if reduced {
        0.0
    } else {
        f64::from(t.stagger_ms / 1000.0) * index as f64
    };
    let p = if start < 0.0 {
        1.0
    } else {
        super::ease((((s.now - start) - stagger) / enter).clamp(0.0, 1.0) as f32)
    };
    (
        p * exit,
        (1.0 - p) * 22.0 + (1.0 - exit) * 8.0,
        (1.0 - p) * 6.0,
    )
}

pub fn keycap(t: &Tokens, label: &str) -> El {
    let ink = t.text;
    el().h(18.0)
        .min_w(18.0)
        .px(5.0)
        .radius(5.0)
        .center()
        .fill(ink.alpha(0.07))
        .border_gradient(1.0, ink.alpha(0.20), ink.alpha(0.07))
        .child(
            text(label)
                .font(TextStyle::new(t.key_size, 600.0).family(Family::SystemMono))
                .color(t.text_dim),
        )
}

pub fn icon_for(action: &Action, size: f32, color: Color) -> El {
    match &action.icon {
        IconRef::Builtin(glyph) => icon(glyph, size).color(color),
        IconRef::Path(path) => {
            icon(custom_icon(path).unwrap_or(&super::icons::POWER), size).color(color)
        }
    }
}

/// User SVG files use the same renderer as bundled icons. The first read is cached; a broken
/// file falls back to the power glyph rather than an invisible button.
fn custom_icon(path: &str) -> Option<&'static Icon> {
    static ICONS: OnceLock<Mutex<HashMap<String, Option<&'static Icon>>>> = OnceLock::new();
    let cache = ICONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(found) = cache.lock().unwrap().get(path) {
        return *found;
    }
    let parsed = parse_svg_icon(path);
    cache.lock().unwrap().insert(path.to_owned(), parsed);
    parsed
}

fn parse_svg_icon(path: &str) -> Option<&'static Icon> {
    let text = std::fs::read_to_string(path).ok()?;
    let start = text.find("<svg")?;
    let open_end = text[start..].find('>')? + start + 1;
    let tag = text.get(start..open_end)?;
    let close = text.rfind("</svg>").unwrap_or(text.len());
    let inner = text.get(open_end..close)?.trim();
    if inner.is_empty() {
        return None;
    }
    let view = tag
        .split_once("viewBox=\"")
        .and_then(|(_, rest)| rest.split('"').next())
        .and_then(|v| v.split_whitespace().nth(2))
        .and_then(|v| v.parse::<f32>().ok())
        .or_else(|| {
            tag.split_once("width=\"")
                .and_then(|(_, rest)| rest.split('"').next())
                .and_then(|v| v.trim_end_matches("px").parse::<f32>().ok())
        })
        .filter(|v| *v > 0.0)
        .unwrap_or(24.0);
    let name: &'static str = Box::leak(path.to_owned().into_boxed_str());
    let body: &'static str = Box::leak(inner.to_owned().into_boxed_str());
    Some(Box::leak(Box::new(Icon { name, view, body })))
}
