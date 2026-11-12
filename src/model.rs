//! The shared state behind the view: one context per launch, one menu state per surface, and
//! the handles that let every surface leave together.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rune::{AppHandle, OverlayHandle};

use crate::actions::{self, Action, Armed, Mode, Press};
use crate::config::Config;
use crate::problem::Problem;
use crate::theme::{Appearance, Layout, Theme, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Menu,
    Dim,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub config: Arc<Config>,
    pub actions: Vec<Action>,
    pub problem: Option<Problem>,
    pub mode: Mode,
    /// Hyprland blurs the `dusk` layer; without it the veil is denser so text stays readable.
    pub blur: bool,
}

static CONTEXT: OnceLock<Mutex<Context>> = OnceLock::new();
static HANDLES: OnceLock<Arc<Handles>> = OnceLock::new();
static APP: OnceLock<AppHandle<MenuState>> = OnceLock::new();

pub fn set_context(context: Context) {
    let slot = CONTEXT.get_or_init(|| Mutex::new(context.clone()));
    *slot.lock().unwrap() = context;
}

pub fn context() -> Context {
    CONTEXT
        .get()
        .map(|c| c.lock().unwrap().clone())
        .unwrap_or_else(|| Context {
            config: Arc::new(Config::default()),
            actions: Vec::new(),
            problem: None,
            mode: Mode::Normal,
            blur: true,
        })
}

pub fn set_handles(handles: Arc<Handles>) {
    let _ = HANDLES.set(handles);
}

pub fn set_app(app: AppHandle<MenuState>) {
    let _ = APP.set(app);
}

pub fn handles() -> Option<Arc<Handles>> {
    HANDLES.get().cloned()
}

fn app() -> Option<&'static AppHandle<MenuState>> {
    APP.get()
}

#[derive(Default)]
pub struct Handles {
    pub menus: Mutex<Vec<OverlayHandle<MenuState>>>,
    pub dims: Mutex<Vec<OverlayHandle<MenuState>>>,
    pub exiting: AtomicBool,
}

impl Handles {
    pub fn is_empty(&self) -> bool {
        self.menus.lock().unwrap().is_empty() && self.dims.lock().unwrap().is_empty()
    }

    pub fn add_menu(&self, handle: OverlayHandle<MenuState>) {
        self.menus.lock().unwrap().push(handle);
    }

    pub fn add_dim(&self, handle: OverlayHandle<MenuState>) {
        self.dims.lock().unwrap().push(handle);
    }

    /// Ask every surface to play its exit and then leave the process. `action` is the thing to
    /// run; `None` is a plain close.
    pub fn begin_exit(&self, state: &MenuState, action: Option<Action>) {
        if self.exiting.swap(true, Ordering::AcqRel) {
            return;
        }
        let selected = action.as_ref().map(|a| a.id.clone());
        let mode = state.mode;
        let exit_ms = state.tokens(true).exit_ms;
        let now = state.now;
        for handle in self
            .menus
            .lock()
            .unwrap()
            .iter()
            .chain(self.dims.lock().unwrap().iter())
        {
            let chosen = selected.clone();
            handle.update(move |s, tx| {
                s.exit = Some(now.max(s.now));
                s.chosen = chosen.clone();
                s.armed = None;
                tx.wake_after(std::time::Duration::from_millis(16));
            });
        }

        let exit_ms = exit_ms.max(40.0) as u64;
        let app = app().cloned();
        let menus = self.menus.lock().unwrap().clone();
        let dims = self.dims.lock().unwrap().clone();
        std::thread::Builder::new()
            .name("dusk-exit".into())
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(exit_ms + 24));
                for handle in menus.iter().chain(dims.iter()) {
                    handle.hide();
                }
                // Let the compositor see the surfaces gone before a power action runs.
                std::thread::sleep(std::time::Duration::from_millis(40));
                if let Some(action) = action
                    && let Err(error) = actions::execute(&action, mode)
                {
                    actions::notify_failure(&action, &error);
                }
                std::thread::sleep(std::time::Duration::from_millis(60));
                if let Some(app) = app {
                    app.quit();
                } else {
                    std::process::exit(0);
                }
            })
            .ok();
    }

    /// Replace the live config in every open surface (preview).
    pub fn replace(&self, config: Arc<Config>, resolved: Vec<Action>, problem: Option<Problem>) {
        for handle in self
            .menus
            .lock()
            .unwrap()
            .iter()
            .chain(self.dims.lock().unwrap().iter())
        {
            let config = config.clone();
            let resolved = resolved.clone();
            let problem = problem.clone();
            handle.update(move |s, tx| {
                s.config = config;
                s.actions = resolved;
                s.problem = problem;
                s.selected = s.selected.min(s.actions.len().saturating_sub(1));
                s.armed = None;
                tx.instant();
            });
        }
    }
}

pub struct MenuState {
    pub config: Arc<Config>,
    pub actions: Vec<Action>,
    pub role: Role,
    pub selected: usize,
    pub armed: Option<Armed>,
    pub now: f64,
    /// When the entrance started; `None` until the first poll in a live run, `Some(-1)` settled.
    pub start: Option<f64>,
    pub exit: Option<f64>,
    pub chosen: Option<String>,
    pub keyboard: bool,
    pub had_focus: bool,
    pub problem: Option<Problem>,
    pub inhibited: Option<String>,
    pub mode: Mode,
    pub handles: Arc<Handles>,
    pub theme_override: Option<Theme>,
    pub layout_override: Option<Layout>,
    pub appearance_override: Option<Appearance>,
    pub last_wall: Option<std::time::SystemTime>,
    pub reduced_override: bool,
    /// The header's time and date.
    pub clock: (String, String),
    pub blur: bool,
}

impl MenuState {
    pub fn new(role: Role) -> Self {
        let ctx = context();
        let animating_script =
            rune::tooling::headless_run().is_some_and(|run| !run.events().is_empty());
        let start = if animating_script {
            None
        } else if rune::is_headless() {
            Some(-1.0)
        } else {
            None
        };
        Self {
            selected: 0,
            config: ctx.config,
            actions: ctx.actions,
            role,
            armed: None,
            now: 0.0,
            start,
            exit: None,
            chosen: None,
            keyboard: false,
            had_focus: false,
            problem: ctx.problem,
            inhibited: None,
            mode: ctx.mode,
            handles: HANDLES.get().cloned().unwrap_or_default(),
            theme_override: None,
            layout_override: None,
            appearance_override: None,
            last_wall: None,
            reduced_override: false,
            clock: crate::ui::clock::now(),
            blur: ctx.blur,
        }
    }

    pub fn theme(&self) -> &Theme {
        self.theme_override.as_ref().unwrap_or(&self.config.theme)
    }

    pub fn layout(&self) -> Layout {
        self.layout_override
            .unwrap_or_else(|| self.config.effective_layout())
    }

    pub fn is_dark(&self, system_dark: bool) -> bool {
        match self.appearance_override.unwrap_or(self.config.appearance) {
            Appearance::Dark => true,
            Appearance::Light => false,
            Appearance::Auto => system_dark,
        }
    }

    pub fn tokens(&self, dark: bool) -> &Tokens {
        self.theme().tokens(dark)
    }

    pub fn selected_action(&self) -> Option<&Action> {
        self.actions.get(self.selected)
    }

    pub fn press_selected(&mut self, tx: &mut rune::Tx) {
        let Some(action) = self.selected_action().cloned() else {
            return;
        };
        self.press(action, tx);
    }

    pub fn press(&mut self, action: Action, tx: &mut rune::Tx) {
        if self.exit.is_some() {
            return;
        }
        match actions::press(&action, self.armed.as_ref(), self.now) {
            Press::Ignore => {}
            Press::Arm(armed) => {
                self.armed = Some(armed);
                tx.wake_after(std::time::Duration::from_millis(16));
            }
            Press::Activate => {
                self.armed = None;
                self.activate(action, tx);
            }
        }
    }

    pub fn activate(&mut self, action: Action, tx: &mut rune::Tx) {
        if self.mode == Mode::Preview {
            eprintln!("dusk: preview would run {}: {}", action.id, action.command);
            self.armed = None;
            return;
        }
        if let Some(what) = power_what(&action)
            && let Some(blocker) = crate::system::blocker(what)
        {
            self.inhibited = Some(blocker.describe());
            tx.wake_after(std::time::Duration::from_millis(16));
            return;
        }
        if self.handles.is_empty() {
            if let Err(error) = actions::execute(&action, self.mode) {
                actions::notify_failure(&action, &error);
            }
            tx.quit();
            return;
        }
        self.exit = Some(self.now.max(0.0));
        self.chosen = Some(action.id.clone());
        self.handles.begin_exit(self, Some(action));
    }

    pub fn close(&mut self, tx: &mut rune::Tx) {
        if self.exit.is_some() {
            return;
        }
        if self.handles.is_empty() {
            tx.quit();
            return;
        }
        self.exit = Some(self.now.max(0.0));
        self.handles.begin_exit(self, None);
    }

    pub fn move_selection(&mut self, delta: isize, tx: &mut rune::Tx) {
        if self.actions.is_empty() {
            return;
        }
        let n = self.actions.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
        self.keyboard = true;
        if let Some(action) = self.actions.get(self.selected) {
            tx.scroll_into_view(format!("dusk/card/{}", action.id));
        }
        tx.wake_after(std::time::Duration::from_millis(16));
    }

    pub fn select(&mut self, index: usize, tx: &mut rune::Tx) {
        if index < self.actions.len() {
            self.selected = index;
            if let Some(action) = self.actions.get(index) {
                tx.scroll_into_view(format!("dusk/card/{}", action.id));
            }
            tx.wake_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn action_by_key(&self, key: char) -> Option<usize> {
        let key = key.to_ascii_lowercase();
        self.actions.iter().position(|a| a.key == Some(key))
    }

    pub fn press_key(&mut self, key: char, tx: &mut rune::Tx) -> bool {
        if let Some(index) = self.action_by_key(key) {
            self.selected = index;
            self.keyboard = true;
            self.press_selected(tx);
            true
        } else {
            false
        }
    }
}

/// `None` for actions that aren't power/sleep, else the logind inhibitor kind to check.
fn power_what(action: &Action) -> Option<&'static str> {
    match action.id.as_str() {
        "shutdown" | "reboot" => Some("shutdown"),
        "suspend" | "hibernate" => Some("sleep"),
        _ => None,
    }
}
