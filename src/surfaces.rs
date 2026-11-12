//! One overlay per display. The focused monitor gets the menu; the rest get a dim-only surface
//! where a click closes dusk. Every surface is built from the same state and leaves together.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rune::{
    App, AppSpec, DisplayTarget, Key, KeyEvent, KeyPolicy, OverlayInput, OverlayLevel,
    OverlayOptions, OverlayPlacement, OverlaySpec, RunOptions, Tx, WindowSpec,
};

use crate::actions::{Action, Armed, Mode};
use crate::compositor::Compositor;
use crate::config::Config;
use crate::model::{self, Context, Handles, MenuState, Role};
use crate::problem::Problem;
use crate::theme::{Layout, Theme};
use crate::ui;
use crate::watchdog;

#[derive(Clone)]
pub struct Launch {
    pub config: Arc<Config>,
    pub actions: Vec<Action>,
    pub problem: Option<Problem>,
    pub mode: Mode,
    pub path: Option<PathBuf>,
    pub caps: crate::system::Capabilities,
    pub compositor: Compositor,
    pub locker: Option<String>,
    pub logout: Option<String>,
    pub instance: Option<Arc<crate::instance::Instance>>,
}

pub fn run(launch: Launch) -> Result<(), String> {
    model::set_context(Context {
        config: launch.config.clone(),
        actions: launch.actions.clone(),
        problem: launch.problem.clone(),
        mode: launch.mode,
        blur: rune::is_headless() || crate::system::blur_rule_present(),
    });
    if !rune::is_headless() {
        watchdog::start();
    }
    let reload = launch.clone();
    // The instance lock and its socket must live as long as the event loop.
    let _instance_keepalive = launch.instance.clone();
    rune::try_run_windows_with(RunOptions { accessory: true }, move |displays| {
        let handles = Arc::new(Handles::default());
        model::set_handles(handles.clone());
        crate::timing::mark("factory");
        let mut app = App::new(spec()).windowless();
        let app_handle = app.handle();
        model::set_app(app_handle.clone());
        if let Some(instance) = &launch.instance {
            let toggle_app = app_handle.clone();
            instance.serve(Arc::new(move || toggle_app.quit()));
        }

        let focused = launch.compositor.focused_monitor();
        let menu_display = focused
            .as_deref()
            .and_then(|name| {
                displays
                    .iter()
                    .position(|d| d.name.as_deref() == Some(name))
            })
            .or_else(|| displays.iter().position(|d| d.primary))
            .unwrap_or(0);

        for (index, display) in displays.iter().enumerate() {
            let role = if index == menu_display {
                Role::Menu
            } else {
                Role::Dim
            };
            let target = display
                .id
                .clone()
                .map_or(DisplayTarget::Primary, DisplayTarget::Id);
            let (w, h) = (
                display.size.0 as f32 / display.scale as f32,
                display.size.1 as f32 / display.scale as f32,
            );
            let view = if role == Role::Menu {
                ui::menu_view
            } else {
                ui::dim_view
            };
            let spec = OverlaySpec::new("dusk", (w, h), view).options(OverlayOptions {
                placement: Some(OverlayPlacement::FullScreen),
                display: target,
                level: OverlayLevel::AboveStatus,
                input: OverlayInput::All,
                keys: if role == Role::Menu {
                    KeyPolicy::OnDemand
                } else {
                    KeyPolicy::None
                },
                prewarm: true,
                ..Default::default()
            });
            let spec = spec.poll(poll).on_focus(on_focus);
            let spec = if role == Role::Menu {
                spec.on_key(on_key)
            } else {
                spec
            };
            let handle = app.overlay(spec, MenuState::new(role));
            if role == Role::Menu {
                handles.add_menu(handle.clone());
            } else {
                handles.add_dim(handle.clone());
            }
            handle.show();
            if role == Role::Menu {
                handle.request_keys();
            }
        }

        crate::timing::mark("overlays");
        // A missing layer-shell (or a surface that failed to open) is reported rather than
        // leaving a process with nothing on screen.
        let watch = handles.menus.lock().unwrap().first().cloned();
        if let Some(menu) = watch.filter(|_| !rune::is_headless()) {
            let app = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(600));
                if menu.availability() == rune::Availability::Unavailable {
                    let reason = menu.failure().unwrap_or_else(|| {
                        "the compositor did not create the layer surface".into()
                    });
                    crate::system::notify("dusk cannot open", &reason);
                    app.quit();
                    std::thread::sleep(Duration::from_millis(100));
                    std::process::exit(crate::EXIT_RENDER);
                }
            });
        }

        if launch.mode == Mode::Preview {
            crate::preview::start(reload);
        }

        vec![WindowSpec::from_app("dusk", app)]
    })
    .map_err(|error| format!("could not start the menu: {error}"))
}

fn spec() -> AppSpec<MenuState> {
    AppSpec {
        scenes: SCENES,
        on_key: Some(on_key),
        poll: Some(poll),
        ..AppSpec::new("dusk", (1920.0, 1080.0), menu_init, ui::menu_view)
    }
}

fn menu_init() -> MenuState {
    MenuState::new(Role::Menu)
}

fn poll(s: &mut MenuState, t: f64, tx: &mut Tx) -> bool {
    crate::timing::mark_once("first poll");
    watchdog::beat();
    let mut changed = false;
    if s.start.is_none() {
        s.start = Some(t);
        changed = true;
    }
    s.now = t;
    // A suspend/resume jumps the wall clock while the monotonic frame clock pauses. Close the
    // menu so it isn't still on screen after the machine wakes.
    let wall = std::time::SystemTime::now();
    if let Some(last) = s.last_wall
        && wall
            .duration_since(last)
            .is_ok_and(|d| d > Duration::from_secs(5))
        && s.exit.is_none()
    {
        s.close(tx);
    }
    s.last_wall = Some(wall);
    let clock = crate::ui::clock::now();
    if clock != s.clock {
        s.clock = clock;
        changed = true;
    }
    if let Some(armed) = s.armed.clone()
        && t > armed.deadline
    {
        s.armed = None;
        changed = true;
    }
    let tokens = s.tokens(true);
    let enter = f64::from(tokens.enter_ms + tokens.stagger_ms * s.actions.len() as f32) / 1000.0;
    let animating = s.exit.is_some()
        || s.armed.is_some()
        || s.start
            .is_some_and(|start| start >= 0.0 && t - start < enter);
    if animating {
        tx.wake_after(Duration::from_millis(16));
        changed = true;
    } else {
        // A once-a-second poll is the watchdog's heartbeat, and costs no frame.
        tx.wake_after(Duration::from_secs(1));
    }
    changed
}

fn on_focus(s: &mut MenuState, focused: bool, tx: &mut Tx) {
    if s.role != Role::Menu || s.exit.is_some() {
        return;
    }
    if focused {
        s.had_focus = true;
    } else if s.had_focus {
        s.close(tx);
    }
}

fn on_key(s: &mut MenuState, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if s.role != Role::Menu || s.exit.is_some() {
        return true;
    }
    // A key held from the launch bind, or the first repeats of one, never activates anything.
    if s.start
        .is_some_and(|start| start >= 0.0 && s.now - start < 0.16)
    {
        return true;
    }
    if tx.key_repeat() {
        return true;
    }
    if ev.mods.cmd || ev.mods.ctrl || ev.mods.alt || ev.mods.logo {
        return true;
    }
    match &ev.key {
        Key::Escape => s.close(tx),
        Key::Left | Key::Up => s.move_selection(-1, tx),
        Key::Right | Key::Down => s.move_selection(1, tx),
        Key::Home => s.select(0, tx),
        Key::End => s.select(s.actions.len().saturating_sub(1), tx),
        Key::Tab if ev.mods.shift => s.move_selection(-1, tx),
        Key::Tab => s.move_selection(1, tx),
        Key::Enter => s.press_selected(tx),
        Key::Text(text) if text == " " => s.press_selected(tx),
        Key::Text(text) if text.chars().count() == 1 => {
            let c = text.chars().next().unwrap_or_default();
            s.keyboard = true;
            s.press_key(c, tx);
        }
        _ => {}
    }
    true
}

fn scene_settled(s: &mut MenuState) {
    s.start = Some(-1.0);
    s.now = 0.0;
}

fn scene_selected(s: &mut MenuState) {
    scene_settled(s);
    s.selected = 2.min(s.actions.len().saturating_sub(1));
    s.keyboard = true;
}

fn scene_armed(s: &mut MenuState) {
    scene_settled(s);
    s.keyboard = true;
    let index = s.actions.iter().position(|a| a.confirm).unwrap_or(0);
    s.selected = index;
    if let Some(action) = s.actions.get(index) {
        s.armed = Some(Armed {
            id: action.id.clone(),
            since: 0.0,
            deadline: 2.4,
        });
    }
}

fn scene_problem(s: &mut MenuState) {
    scene_settled(s);
    s.problem = Some(Problem::new(
        "config.kdl",
        12,
        "unknown token \"radus\"; using the theme value",
    ));
}

fn scene_inhibited(s: &mut MenuState) {
    scene_settled(s);
    s.inhibited = Some("Saving browser state (Brave Browser)".into());
}

fn scene_many(s: &mut MenuState) {
    scene_settled(s);
    let base: Vec<Action> = s.actions.clone();
    if base.is_empty() {
        return;
    }
    for i in 0..6 {
        let mut action = base[i % base.len()].clone();
        action.id = format!("extra-{i}");
        action.label = format!("Action {}", i + 6);
        action.key = Some((b'1' + i as u8) as char);
        action.confirm = false;
        s.actions.push(action);
    }
    s.keyboard = true;
}

fn scene_long_labels(s: &mut MenuState) {
    scene_settled(s);
    let labels = [
        "Suspend until tomorrow morning",
        "Log out of this session",
        "Restart the computer",
        "Shut down and install updates",
    ];
    for (action, label) in s.actions.iter_mut().zip(labels) {
        action.label = label.to_owned();
    }
    s.keyboard = true;
}

fn scene_halo(s: &mut MenuState) {
    scene_settled(s);
    s.theme_override = Theme::builtin("halo").map(|(t, _)| t);
    s.layout_override = Some(Layout::Row);
}

fn scene_mono(s: &mut MenuState) {
    scene_settled(s);
    s.theme_override = Theme::builtin("mono").map(|(t, _)| t);
    s.layout_override = Some(Layout::List);
}

fn scene_reduced(s: &mut MenuState) {
    scene_settled(s);
    s.reduced_override = true;
}

fn scene_light(s: &mut MenuState) {
    scene_settled(s);
    s.appearance_override = Some(crate::theme::Appearance::Light);
}

fn scene_dark(s: &mut MenuState) {
    scene_settled(s);
    s.appearance_override = Some(crate::theme::Appearance::Dark);
}

type Scene = (&'static str, fn(&mut MenuState));

static SCENES: &[Scene] = &[
    ("idle", scene_settled),
    ("selected", scene_selected),
    ("armed", scene_armed),
    ("problem", scene_problem),
    ("inhibited", scene_inhibited),
    ("many", scene_many),
    ("long-labels", scene_long_labels),
    ("halo", scene_halo),
    ("mono", scene_mono),
    ("reduced", scene_reduced),
    ("light", scene_light),
    ("dark", scene_dark),
];

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Key, KeyEvent, Mods};

    #[test]
    fn a_key_from_the_launch_bind_is_ignored_until_the_grace_ends() {
        let mut state = MenuState::new(Role::Menu);
        state.actions = (0..2)
            .map(|i| crate::actions::Action {
                id: format!("extra-{i}"),
                label: format!("Extra {i}"),
                icon: crate::actions::IconRef::Builtin(&crate::ui::icons::POWER),
                command: "true".into(),
                key: Some((b'x' + i as u8) as char),
                confirm: false,
                builtin: None,
            })
            .collect();
        state.start = Some(0.0);
        state.now = 0.05;
        let mut tx = Tx::default();
        assert!(on_key(
            &mut state,
            &KeyEvent {
                key: Key::Right,
                mods: Mods::default()
            },
            &mut tx
        ));
        assert_eq!(state.selected, 0);

        state.now = 0.3;
        assert!(on_key(
            &mut state,
            &KeyEvent {
                key: Key::Right,
                mods: Mods::default()
            },
            &mut tx
        ));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn a_held_modifier_never_triggers_a_mnemonic() {
        let mut state = MenuState::new(Role::Menu);
        state.actions = vec![crate::actions::Action {
            id: "extra".into(),
            label: "Extra".into(),
            icon: crate::actions::IconRef::Builtin(&crate::ui::icons::POWER),
            command: "true".into(),
            key: Some('s'),
            confirm: false,
            builtin: None,
        }];
        state.start = Some(-1.0);
        let mut tx = Tx::default();
        let mods = Mods {
            logo: true,
            ..Mods::default()
        };
        assert!(on_key(
            &mut state,
            &KeyEvent {
                key: Key::Text("s".into()),
                mods
            },
            &mut tx
        ));
        assert_eq!(state.selected, 0);
        assert!(state.exit.is_none());
    }
}
