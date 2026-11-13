//! The view is a pure function of `(config, MenuState)`. Headless scenes, preview and the real
//! overlay all draw this same tree.

mod card;
pub mod clock;
pub mod icons;

use rune::tok::type_;
use rune::{
    Color, Cx, El, Family, ScrollSpec, Shadow, Spring, TextStyle, col, el, row, spacer, text,
};

use crate::model::MenuState;
use crate::theme::{Layout, Tokens};
use card::{LIFT, card, keycap, list_row};

const EDGE: f32 = 64.0;
/// Height kept for the clock and hint line when a row-sized board is present.
const BOARD_RESERVE: f32 = 260.0;

pub fn menu_view(s: &MenuState, cx: &mut Cx<MenuState>) -> El {
    if s.reduced_override {
        cx.reduced_motion(true);
    }
    let dark = s.is_dark(cx.system_dark());
    let t = s.tokens(dark);
    let reduced = cx.is_reduced_motion();
    let (root_w, root_h) = cx
        .layout_size("dusk/root")
        .map_or((1920.0, 1080.0), |v| (v[0], v[1]));
    let ui_scale = fit_ui_scale(
        (root_w / 1920.0).min(root_h / 1080.0).clamp(0.82, 1.8),
        s.actions.len(),
        t,
        s.layout(),
        root_w,
        root_h,
    );
    let (intro, exit) = timings(s, t, reduced);
    let fade = intro * exit;

    let mut root = el()
        .key("dusk/root")
        .w_full()
        .h_full()
        .on_press_tx(|s: &mut MenuState, tx: &mut rune::Tx| s.close(tx))
        .child(backdrop(t, root_w, root_h, fade, s.blur));

    if s.actions.is_empty() {
        return root.child(
            col().w_full().h_full().center().child(
                text("Nothing to do")
                    .font(TextStyle::new(20.0, 600.0))
                    .color(t.text.alpha(fade)),
            ),
        );
    }

    let layout = effective_layout(
        s.actions.len(),
        t,
        s.layout(),
        root_w / ui_scale,
        root_h / ui_scale,
    );
    let board = if t.compact {
        pill(s, t, exit, reduced)
    } else {
        match layout {
            Layout::List => list(s, t, root_w / ui_scale, root_h / ui_scale, exit, reduced),
            Layout::Grid => grid_view(s, t, root_w / ui_scale, root_h / ui_scale, exit, reduced),
            Layout::Row => row_view(s, t, exit, reduced),
        }
    };

    let rise = if reduced { 0.0 } else { (1.0 - intro) * 12.0 };
    let mut stage = col().center().gap(0.0);
    if t.header && !t.compact && layout != Layout::List {
        stage = stage.child(header(s, t).opacity(fade).offset(0.0, rise).mb(44.0));
    }
    stage = stage.child(board);
    if let Some(inhibited) = &s.inhibited {
        stage = stage.child(blocked(t, inhibited).opacity(fade).mt(28.0));
    }
    if !t.compact && layout != Layout::List {
        stage = stage.child(
            hints(t)
                .opacity(fade * if s.keyboard { 1.0 } else { 0.7 })
                .mt(40.0),
        );
    }
    root = root.child(
        col()
            .w_full()
            .h_full()
            .center()
            .child(stage.scale(ui_scale)),
    );

    // One quiet line, so a broken colour never costs you the menu.
    if let Some(problem) = &s.problem {
        root = root.child(
            el().pin(None, None, Some(EDGE), Some(18.0))
                .w_full()
                .center()
                .child(
                    text(format!("{problem}"))
                        .font(type_::LABEL)
                        .color(t.text_dim.alpha(0.85))
                        .truncate()
                        .opacity(fade),
                ),
        );
    }
    if s.mode.prints_actions() {
        let note = match s.mode {
            crate::actions::Mode::Preview => "preview · actions print only",
            _ => "dry run · actions print only",
        };
        root = root.child(
            el().pin(Some(EDGE), None, None, Some(16.0)).child(
                text(note)
                    .font(type_::MONO_CAPTION.family(Family::SystemMono))
                    .color(t.text_dim.alpha(0.7))
                    .opacity(intro),
            ),
        );
    }
    root
}

pub fn dim_view(s: &MenuState, cx: &mut Cx<MenuState>) -> El {
    if s.reduced_override {
        cx.reduced_motion(true);
    }
    let dark = s.is_dark(cx.system_dark());
    let t = s.tokens(dark);
    let reduced = cx.is_reduced_motion();
    let (w, h) = cx
        .layout_size("dusk/dim")
        .map_or((1920.0, 1080.0), |v| (v[0], v[1]));
    let (intro, exit) = timings(s, t, reduced);
    el().key("dusk/dim")
        .w_full()
        .h_full()
        .child(backdrop(t, w, h, intro * exit, s.blur))
        .on_press_tx(|s: &mut MenuState, tx: &mut rune::Tx| s.close(tx))
}

fn timings(s: &MenuState, t: &Tokens, reduced: bool) -> (f32, f32) {
    let now = s.now;
    let start = s.start.unwrap_or(now);
    let enter = if reduced {
        0.15
    } else {
        f64::from((t.enter_ms / 1000.0).max(0.01))
    };
    let intro = if start < 0.0 {
        1.0
    } else {
        ease(((now - start) / enter).clamp(0.0, 1.0) as f32)
    };
    let exit = match s.exit {
        Some(at) => {
            let secs = if reduced {
                0.15
            } else {
                f64::from((t.exit_ms / 1000.0).max(0.01))
            };
            1.0 - ease(((now - at) / secs).clamp(0.0, 1.0) as f32)
        }
        None => 1.0,
    };
    (intro, exit)
}

pub(crate) fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

/// The evening sky: the dim, a vertical indigo-to-ember wash, two soft blooms, a faint glow
/// behind the cards and a vignette. Hyprland's blur, when present, sits under all of it.
fn backdrop(t: &Tokens, w: f32, h: f32, alpha: f32, blur: bool) -> El {
    let layer = || el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0));
    // Unblurred, the desktop's text would fight the cards: thicken the veil instead.
    let dim = if blur {
        t.dim
    } else {
        t.dim.alpha(t.dim.a.max(0.88))
    };
    layer()
        .fill(dim)
        .gradient(t.backdrop_top, t.backdrop_bottom)
        .opacity(alpha)
        .child(layer().radial_gradient(
            [0.12, 0.05],
            [w * 0.62, h * 0.78],
            [(0.0, t.bloom_a), (1.0, Color::TRANSPARENT)],
        ))
        .child(layer().radial_gradient(
            [0.92, 1.0],
            [w * 0.58, h * 0.72],
            [(0.0, t.bloom_b), (1.0, Color::TRANSPARENT)],
        ))
        .child(layer().radial_gradient(
            [0.5, 0.5],
            [w * 0.32, h * 0.34],
            [(0.0, t.accent.alpha(0.07)), (1.0, Color::TRANSPARENT)],
        ))
        .child(layer().radial_gradient(
            [0.5, 0.48],
            [w * 0.82, h * 0.86],
            [
                (0.0, Color::TRANSPARENT),
                (0.68, Color::TRANSPARENT),
                (1.0, t.shadow),
            ],
        ))
}

/// The time, large and light, with the date under it.
fn header(s: &MenuState, t: &Tokens) -> El {
    col()
        .center()
        .gap(6.0)
        .child(
            text(s.clock.0.clone())
                .font(
                    TextStyle::new(64.0, 280.0)
                        .tracking(-1.5)
                        .features("'tnum' 1"),
                )
                .color(t.text),
        )
        .child(
            text(s.clock.1.clone())
                .font(TextStyle::new(15.0, 460.0).tracking(0.2))
                .color(t.text_dim),
        )
}

/// A quiet legend of the navigation keys.
fn hints(t: &Tokens) -> El {
    let label = |s: &'static str| {
        text(s)
            .font(TextStyle::new(12.0, 500.0))
            .color(t.text_dim.alpha(0.8))
    };
    let item = |keys: &[&str], what: &'static str| {
        let mut r = row().gap(4.0).center_y();
        for k in keys {
            r = r.child(keycap(t, k));
        }
        r.child(label(what).ml(4.0))
    };
    row()
        .gap(22.0)
        .center_y()
        .child(item(&["←", "→"], "select"))
        .child(item(&["↵"], "choose"))
        .child(item(&["esc"], "close"))
}

/// A chip naming what is blocking a power action.
fn blocked(t: &Tokens, who: &str) -> El {
    row()
        .gap(10.0)
        .center_y()
        .px(16.0)
        .py(9.0)
        .radius(999.0)
        .fill(t.warning.alpha(0.12))
        .border(1.0, t.warning.alpha(0.32))
        .child(el().size(7.0, 7.0).radius(3.5).fill(t.warning))
        .child(
            text(format!("Blocked by {who}"))
                .font(TextStyle::new(13.0, 520.0))
                .color(t.warning),
        )
}

/// The shape actually drawn. A row that does not fit becomes a grid only when every row still
/// fits vertically; on very short screens a scrolling list is the usable fallback.
/// On a short screen the row can be kept by scaling it down a little. This only kicks in when a
/// grid would not fit vertically either; the floor keeps labels legible.
fn fit_ui_scale(
    ui_scale: f32,
    n: usize,
    t: &Tokens,
    requested: Layout,
    root_w: f32,
    root_h: f32,
) -> f32 {
    if t.compact || requested != Layout::Row {
        return ui_scale;
    }
    let (w, h) = (root_w / ui_scale, root_h / ui_scale);
    let stage_h = t.card_h + BOARD_RESERVE + 2.0 * EDGE;
    if (fits_row(n, t, w) && stage_h <= h) || grid_fits(n, t, w, h) {
        return ui_scale;
    }
    (root_w / (board_width(t, n) + 2.0 * EDGE + 1.0))
        .min(root_h / stage_h)
        .clamp(0.55, ui_scale)
}

fn effective_layout(n: usize, t: &Tokens, requested: Layout, root_w: f32, root_h: f32) -> Layout {
    if t.compact || matches!(requested, Layout::List | Layout::Grid) {
        return requested;
    }
    if fits_row(n, t, root_w) {
        return Layout::Row;
    }
    let cols = cols_for(t, root_w, n);
    if grid_fits(n, t, root_w, root_h) && board_width(t, cols) <= root_w - 2.0 * EDGE {
        Layout::Grid
    } else {
        Layout::List
    }
}

fn fits_row(n: usize, t: &Tokens, root_w: f32) -> bool {
    board_width(t, n) <= root_w - 2.0 * EDGE
}

fn grid_fits(n: usize, t: &Tokens, root_w: f32, root_h: f32) -> bool {
    let cols = cols_for(t, root_w, n);
    let rows = n.div_ceil(cols.max(1));
    let board_h = rows as f32 * t.card_h + rows.saturating_sub(1) as f32 * t.gap;
    board_h <= root_h - 2.0 * EDGE - BOARD_RESERVE
}

fn board_width(t: &Tokens, cols: usize) -> f32 {
    cols as f32 * t.card_w + cols.saturating_sub(1) as f32 * t.gap
}

fn cols_for(t: &Tokens, root_w: f32, n: usize) -> usize {
    let fit = (((root_w - 2.0 * EDGE + t.gap) / (t.card_w + t.gap)).floor() as usize).max(1);
    // Balance rows instead of filling the last one and leaving a widow.
    let rows = n.div_ceil(fit);
    n.div_ceil(rows.max(1)).clamp(1, n.max(1))
}

/// The highlight that springs between cards, glowing in the selected action's hue.
fn highlight(s: &MenuState, t: &Tokens, x: f32, y: f32, w: f32, h: f32, radius: f32) -> El {
    let hue = s.selected_action().map_or(t.accent, |a| card::hue(t, s, a));
    let mono = t.mono;
    el().key("dusk/highlight")
        .absolute(x, y)
        .size(w, h)
        .radius(radius)
        .continuous_corners()
        .fill(t.card)
        .gradient(hue.alpha(if mono { 0.10 } else { 0.24 }), hue.alpha(0.04))
        .border_gradient(
            1.0,
            hue.alpha(if mono { 0.5 } else { 0.8 }),
            hue.alpha(0.18),
        )
        .box_shadow(Shadow {
            blur: if mono { 0.0 } else { 56.0 },
            color: hue.alpha(if mono { 0.0 } else { 0.28 }),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 18.0,
            blur: 40.0,
            color: t.shadow,
            ..Shadow::default()
        })
        .motion(Spring::new(0.34, t.spring_bounce.clamp(0.0, 1.0)))
}

fn board(
    s: &MenuState,
    t: &Tokens,
    cols: usize,
    cells: Vec<El>,
    exit: f32,
    radius: f32,
    lift: f32,
) -> (El, f32) {
    let rows = cells.len().div_ceil(cols.max(1));
    let w = board_width(t, cols);
    let h = rows as f32 * t.card_h + rows.saturating_sub(1) as f32 * t.gap;
    let step = (t.card_w + t.gap, t.card_h + t.gap);
    let (c, r) = (s.selected % cols, s.selected / cols);
    let mut board = el().key("dusk/board").w(w).h(h).child(
        highlight(
            s,
            t,
            c as f32 * step.0,
            r as f32 * step.1 - lift,
            t.card_w,
            t.card_h,
            radius,
        )
        .opacity(exit),
    );
    for (i, cell) in cells.into_iter().enumerate() {
        board = board.child(
            cell.absolute((i % cols) as f32 * step.0, (i / cols) as f32 * step.1)
                .size(t.card_w, t.card_h),
        );
    }
    (board, h)
}

fn cards(s: &MenuState, t: &Tokens, exit: f32, reduced: bool, compact: bool) -> Vec<El> {
    s.actions
        .iter()
        .enumerate()
        .map(|(i, a)| card(s, t, i, a, exit, reduced, compact))
        .collect()
}

fn row_view(s: &MenuState, t: &Tokens, exit: f32, reduced: bool) -> El {
    let lift = if reduced { 0.0 } else { LIFT };
    board(
        s,
        t,
        s.actions.len().max(1),
        cards(s, t, exit, reduced, false),
        exit,
        t.radius,
        lift,
    )
    .0
}

fn grid_view(s: &MenuState, t: &Tokens, root_w: f32, root_h: f32, exit: f32, reduced: bool) -> El {
    let cols = cols_for(t, root_w, s.actions.len());
    let lift = if reduced { 0.0 } else { LIFT };
    let (b, board_h) = board(
        s,
        t,
        cols,
        cards(s, t, exit, reduced, false),
        exit,
        t.radius,
        lift,
    );
    if board_h > root_h - 2.0 * EDGE - BOARD_RESERVE {
        col()
            .max_h((root_h - 2.0 * EDGE - BOARD_RESERVE).max(t.card_h))
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(b)
    } else {
        b
    }
}

fn pill(s: &MenuState, t: &Tokens, exit: f32, reduced: bool) -> El {
    let cols = s.actions.len().max(1);
    let w = board_width(t, cols);
    let (inner, _) = board(
        s,
        t,
        cols,
        cards(s, t, exit, reduced, true),
        exit,
        t.card_h / 2.0,
        0.0,
    );
    let (gw, gh) = (w + 2.0 * t.padding, t.card_h + 2.0 * t.padding);
    let edge = t.card_border;
    let glass = el()
        .absolute(-t.padding, -t.padding)
        .size(gw, gh)
        .radius(gh / 2.0)
        .fill(t.card)
        .gradient(edge.alpha(edge.a * 0.5), Color::TRANSPARENT)
        .border_gradient(
            1.0,
            edge.alpha((edge.a * 1.6).min(1.0)),
            edge.alpha(edge.a * 0.4),
        )
        .box_shadow(Shadow {
            y: 2.0,
            blur: 8.0,
            color: t.shadow.alpha(t.shadow.a * 0.5),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 22.0,
            blur: 54.0,
            color: t.shadow,
            ..Shadow::default()
        })
        .opacity(exit);
    el().w(w).h(t.card_h).child(glass).child(inner)
}

fn list(s: &MenuState, t: &Tokens, root_w: f32, root_h: f32, exit: f32, reduced: bool) -> El {
    let row_h = t.card_h;
    let available = (root_w - 2.0 * EDGE).max(1.0);
    let w = if t.mono {
        t.card_w.min(available)
    } else {
        available.min(640.0)
    };
    let n = s.actions.len();
    let h = n as f32 * row_h + n.saturating_sub(1) as f32 * t.gap;
    let mut inner = el().key("dusk/board").w(w).h(h).child(
        highlight(
            s,
            t,
            0.0,
            s.selected as f32 * (row_h + t.gap),
            w,
            row_h,
            t.radius,
        )
        .opacity(exit),
    );
    for (i, a) in s.actions.iter().enumerate() {
        inner = inner.child(
            list_row(s, t, i, a, w, row_h, exit, reduced).absolute(0.0, i as f32 * (row_h + t.gap)),
        );
    }
    let mono = |size: f32| TextStyle::new(size, 500.0).family(Family::SystemMono);
    let header = row()
        .w(w)
        .h(22.0)
        .px(2.0)
        .center_y()
        .child(text("dusk").font(mono(11.5)).color(t.text_dim.alpha(0.7)))
        .child(spacer())
        .child(
            text(format!("{} · esc closes", s.clock.0))
                .font(mono(11.0))
                .color(t.text_dim.alpha(0.55)),
        );
    let all = col().w(w).gap(6.0).child(header.opacity(exit)).child(inner);
    if h + 40.0 > root_h - 2.0 * EDGE {
        col()
            .max_h(root_h - 2.0 * EDGE)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(all)
    } else {
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_screen_shrinks_the_row_instead_of_clipping() {
        let t = Tokens::default();
        let scale = fit_ui_scale(0.82, 5, &t, Layout::Row, 640.0, 320.0);
        assert!(scale < 0.82);
        assert!(scale >= 0.55);
        assert!(fits_row(5, &t, 640.0 / scale));
    }

    #[test]
    fn a_short_wide_screen_shrinks_to_fit_vertically() {
        let t = Tokens::default();
        let scale = fit_ui_scale(0.82, 5, &t, Layout::Row, 1920.0, 400.0);
        assert!(scale < 0.82);
        assert!((t.card_h + BOARD_RESERVE + 2.0 * EDGE) * scale <= 400.0);
    }

    #[test]
    fn an_over_wide_grid_card_falls_back_to_a_list() {
        let t = Tokens {
            card_w: 900.0,
            ..Tokens::default()
        };
        assert_eq!(
            effective_layout(1, &t, Layout::Row, 640.0, 1000.0),
            Layout::List
        );
    }

    #[test]
    fn a_short_narrow_row_falls_back_to_a_list() {
        let t = Tokens::default();
        // 1280x720 at 2x: the five-card row does not fit, and neither do two grid rows.
        assert_eq!(
            effective_layout(5, &t, Layout::Row, 640.0, 320.0),
            Layout::List
        );
    }

    #[test]
    fn a_portrait_screen_keeps_the_many_action_grid() {
        let t = Tokens::default();
        assert_eq!(
            effective_layout(11, &t, Layout::Row, 800.0, 1000.0),
            Layout::Grid
        );
    }

    #[test]
    fn a_wide_screen_keeps_the_row() {
        let t = Tokens::default();
        assert_eq!(
            effective_layout(5, &t, Layout::Row, 1920.0, 1080.0),
            Layout::Row
        );
    }

    #[test]
    fn explicit_layouts_are_not_overridden() {
        let t = Tokens::default();
        assert_eq!(
            effective_layout(5, &t, Layout::Grid, 640.0, 320.0),
            Layout::Grid
        );
        assert_eq!(
            effective_layout(5, &t, Layout::List, 1920.0, 1080.0),
            Layout::List
        );
    }
}
