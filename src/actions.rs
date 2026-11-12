//! Built-in actions, their order, mnemonic keys, capabilities, confirmation state and spawn.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rune::Icon;

use crate::compositor::Compositor;
use crate::config::{ActionPatch, Config};
use crate::problem::Problem;
use crate::system::Capabilities;
use crate::ui::icons;

/// How long a destructive action waits for a second press.
pub const CONFIRM_WINDOW: f64 = 3.0;
/// A second press inside this gap is a repeat or a double tap, never a confirmation.
pub const CONFIRM_GAP: f64 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    /// The real menu, but actions only print.
    DryRun,
    /// Live-reloading preview, actions only print.
    Preview,
}

impl Mode {
    pub fn prints_actions(self) -> bool {
        !matches!(self, Mode::Normal)
    }
}

#[derive(Clone, Debug)]
pub enum IconRef {
    Builtin(&'static Icon),
    Path(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Lock,
    Logout,
    Suspend,
    Hibernate,
    Reboot,
    Shutdown,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub icon: IconRef,
    pub command: String,
    pub key: Option<char>,
    pub confirm: bool,
    pub builtin: Option<Builtin>,
}

impl Action {
    pub fn hue_id(&self) -> &str {
        &self.id
    }
}

pub const BUILTINS: [(&str, &str, Builtin); 6] = [
    ("lock", "Lock", Builtin::Lock),
    ("logout", "Log out", Builtin::Logout),
    ("suspend", "Suspend", Builtin::Suspend),
    ("hibernate", "Hibernate", Builtin::Hibernate),
    ("reboot", "Reboot", Builtin::Reboot),
    ("shutdown", "Shut down", Builtin::Shutdown),
];

fn builtin_icon(id: &str) -> &'static Icon {
    match id {
        "lock" => &icons::LOCK,
        "logout" => &icons::LOGOUT,
        "suspend" => &icons::SUSPEND,
        "hibernate" => &icons::HIBERNATE,
        "reboot" => &icons::REBOOT,
        _ => &icons::POWER,
    }
}

fn default_key(id: &str) -> Option<char> {
    match id {
        "lock" => Some('l'),
        "logout" => Some('e'),
        "suspend" => Some('s'),
        "hibernate" => Some('h'),
        "reboot" => Some('r'),
        "shutdown" => Some('p'),
        _ => None,
    }
}

fn default_confirm(id: &str) -> bool {
    matches!(id, "logout" | "reboot" | "shutdown")
}

/// Resolve the built-ins, hide/order settings and config overrides into the list the view draws.
pub fn resolve(
    config: &Config,
    caps: &Capabilities,
    compositor: Compositor,
    locker: Option<&str>,
    logout: Option<&str>,
    file: &str,
    problems: &mut Vec<Problem>,
) -> Vec<Action> {
    let defaults = default_actions(locker, logout);
    let mut actions: Vec<Action> = Vec::new();
    for mut action in defaults {
        let id = action.id.clone();
        let patch = config.actions.iter().find(|p| p.id == id);
        apply_patch(&mut action, patch, problems, file);
        let forced = patch
            .and_then(|p| p.run.as_deref())
            .is_some_and(|run| !run.trim().is_empty());
        let available = match action.builtin {
            Some(Builtin::Lock) => true,
            Some(Builtin::Logout) => compositor.supports_logout() && !action.command.is_empty(),
            Some(Builtin::Suspend) => forced || caps.suspend.allows(),
            Some(Builtin::Hibernate) => forced || caps.hibernate.allows(),
            Some(Builtin::Reboot) => forced || caps.reboot.allows(),
            Some(Builtin::Shutdown) => forced || caps.power_off.allows(),
            None => true,
        };
        if available && !config.hide.iter().any(|h| h == &action.id) {
            actions.push(action);
        }
    }

    // Custom actions are every patch whose id is not a built-in.
    for patch in &config.actions {
        if BUILTINS.iter().any(|(id, ..)| *id == patch.id) {
            continue;
        }
        let Some(label) = patch.label.clone() else {
            problems.push(Problem::new(
                file,
                patch.line,
                format!("custom action \"{}\" needs a label", patch.id),
            ));
            continue;
        };
        let Some(command) = patch.run.clone() else {
            problems.push(Problem::new(
                file,
                patch.line,
                format!("custom action \"{}\" needs a run command", patch.id),
            ));
            continue;
        };
        if config.hide.iter().any(|h| h == &patch.id) {
            continue;
        }
        let icon = match patch.icon.as_deref() {
            Some(path) if path.starts_with('~') || path.contains('/') => {
                IconRef::Path(expand_path(path))
            }
            Some(name) => IconRef::Builtin(icon_by_name(name).unwrap_or(&icons::POWER)),
            None => IconRef::Builtin(&icons::POWER),
        };
        actions.push(Action {
            id: patch.id.clone(),
            label,
            icon,
            command,
            key: patch.key.flatten().map(|c| c.to_ascii_lowercase()),
            confirm: patch.confirm.unwrap_or(false),
            builtin: None,
        });
    }

    for action in &mut actions {
        if let IconRef::Path(path) = &action.icon
            && !std::path::Path::new(path).exists()
        {
            let line = config
                .actions
                .iter()
                .find(|p| p.id == action.id)
                .map_or(1, |p| p.line);
            problems.push(Problem::new(
                file,
                line,
                format!("icon \"{path}\" was not found; using the power icon"),
            ));
            action.icon = IconRef::Builtin(&icons::POWER);
        }
    }

    // Order: the ids named first, in that order, then whatever remains.
    if !config.order.is_empty() {
        let mut ordered = Vec::with_capacity(actions.len());
        for id in &config.order {
            if let Some(i) = actions.iter().position(|a| &a.id == id) {
                ordered.push(actions.remove(i));
            }
        }
        ordered.append(&mut actions);
        actions = ordered;
    }

    validate_keys(&mut actions, config, file, problems);
    actions
}

fn default_actions(locker: Option<&str>, logout: Option<&str>) -> Vec<Action> {
    BUILTINS
        .iter()
        .map(|(id, label, builtin)| {
            let command = match builtin {
                Builtin::Lock => {
                    locker.map_or_else(|| "loginctl lock-session".to_owned(), str::to_owned)
                }
                Builtin::Logout => logout.unwrap_or_default().to_owned(),
                Builtin::Suspend => "systemctl suspend".to_owned(),
                Builtin::Hibernate => "systemctl hibernate".to_owned(),
                Builtin::Reboot => "systemctl reboot".to_owned(),
                Builtin::Shutdown => "systemctl poweroff".to_owned(),
            };
            Action {
                id: (*id).to_owned(),
                label: (*label).to_owned(),
                icon: IconRef::Builtin(builtin_icon(id)),
                command,
                key: default_key(id),
                confirm: default_confirm(id),
                builtin: Some(*builtin),
            }
        })
        .collect()
}

fn apply_patch(
    action: &mut Action,
    patch: Option<&ActionPatch>,
    problems: &mut Vec<Problem>,
    file: &str,
) {
    let Some(patch) = patch else { return };
    if let Some(label) = &patch.label {
        action.label = label.clone();
    }
    if let Some(icon) = &patch.icon {
        action.icon = if icon.starts_with('~') || icon.contains('/') {
            IconRef::Path(expand_path(icon))
        } else {
            IconRef::Builtin(icon_by_name(icon).unwrap_or_else(|| builtin_icon(&action.id)))
        };
    }
    if let Some(run) = &patch.run {
        action.command = run.clone();
    }
    if let Some(key) = patch.key {
        action.key = key;
    }
    if let Some(confirm) = patch.confirm {
        action.confirm = confirm;
    }
    let _ = (problems, file);
}

fn icon_by_name(name: &str) -> Option<&'static Icon> {
    Some(match name {
        "lock" => &icons::LOCK,
        "logout" | "exit" => &icons::LOGOUT,
        "suspend" | "moon" => &icons::SUSPEND,
        "hibernate" => &icons::HIBERNATE,
        "reboot" | "restart" => &icons::REBOOT,
        "shutdown" | "power" => &icons::POWER,
        _ => return None,
    })
}

fn expand_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return format!("{}/{}", home.to_string_lossy(), rest);
    }
    path.to_owned()
}

fn validate_keys(actions: &mut [Action], config: &Config, file: &str, problems: &mut Vec<Problem>) {
    const RESERVED: &[char] = &['\t', '\n', '\r', ' '];
    let mut used: Vec<(char, String)> = Vec::new();
    for action in actions.iter_mut() {
        let Some(key) = action.key else { continue };
        let line = config
            .actions
            .iter()
            .find(|p| p.id == action.id)
            .map_or(1, |p| p.line);
        if RESERVED.contains(&key) {
            problems.push(Problem::new(
                file,
                line,
                format!("key \"{key}\" is reserved for navigation"),
            ));
            action.key = None;
            continue;
        }
        if let Some((_, other)) = used.iter().find(|(k, _)| *k == key) {
            problems.push(Problem::new(
                file,
                line,
                format!("key \"{key}\" is already used by \"{other}\"; this one is ignored"),
            ));
            action.key = None;
            continue;
        }
        used.push((key, action.id.clone()));
    }
}

/// Where a confirmed action stands after a press.
#[derive(Clone, Debug, PartialEq)]
pub struct Armed {
    pub id: String,
    pub since: f64,
    pub deadline: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Press {
    Ignore,
    Arm(Armed),
    Activate,
}

/// The pure confirmation rule. `now` is the menu's monotonic seconds.
pub fn press(action: &Action, armed: Option<&Armed>, now: f64) -> Press {
    if !action.confirm {
        return Press::Activate;
    }
    if let Some(armed) = armed.filter(|a| a.id == action.id) {
        if now - armed.since < CONFIRM_GAP {
            return Press::Ignore;
        }
        if now <= armed.deadline {
            return Press::Activate;
        }
    }
    Press::Arm(Armed {
        id: action.id.clone(),
        since: now,
        deadline: now + CONFIRM_WINDOW,
    })
}

pub fn armed_label(action: &Action) -> String {
    match action.builtin {
        Some(Builtin::Logout) => "Press again to log out".into(),
        Some(Builtin::Reboot) => "Press again to reboot".into(),
        Some(Builtin::Shutdown) => "Press again to shut down".into(),
        _ => format!("Press again to {}", action.label.to_lowercase()),
    }
}

/// Spawn an action detached, reporting a quick failure. A command still running after 600 ms is
/// the normal case for `systemctl suspend` and desktop shutdown helpers.
pub fn execute(action: &Action, mode: Mode) -> Result<(), String> {
    if mode.prints_actions() {
        eprintln!("dusk: would run {}: {}", action.id, action.command);
        return Ok(());
    }
    eprintln!("dusk: running {}: {}", action.id, action.command);
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&action.command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    // SAFETY: setsid is async-signal-safe and only touches the child between fork and exec.
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start `{}`: {e}", action.command))?;
    let deadline = Instant::now() + Duration::from_millis(600);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => {
                return Err(format!("`{}` exited with {status}", action.command));
            }
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => return Ok(()),
            Err(err) => return Err(format!("waiting for `{}`: {err}", action.command)),
        }
    }
}

pub fn notify_failure(action: &Action, error: &str) {
    eprintln!("dusk: {error}");
    let mut child = match Command::new("notify-send")
        .arg("dusk")
        .arg(format!("{} failed", action.label))
        .arg(error)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return,
    };
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::Capability;

    fn action(confirm: bool) -> Action {
        Action {
            id: "shutdown".into(),
            label: "Shut down".into(),
            icon: IconRef::Builtin(&icons::POWER),
            command: "true".into(),
            key: Some('p'),
            confirm,
            builtin: Some(Builtin::Shutdown),
        }
    }

    #[test]
    fn a_destructive_action_arms_then_fires_inside_the_window() {
        let a = action(true);
        let Press::Arm(armed) = press(&a, None, 0.0) else {
            panic!("first press arms")
        };
        assert_eq!(
            press(&a, Some(&armed), 0.1),
            Press::Ignore,
            "a double tap is not a confirmation"
        );
        assert_eq!(press(&a, Some(&armed), 0.3), Press::Activate);
        assert_eq!(
            press(&a, Some(&armed), 3.1),
            Press::Arm(Armed {
                id: "shutdown".into(),
                since: 3.1,
                deadline: 6.1
            })
        );
    }

    #[test]
    fn a_safe_action_fires_at_once() {
        assert_eq!(press(&action(false), None, 0.0), Press::Activate);
    }

    #[test]
    fn a_different_action_arms_instead_of_confirming() {
        let a = action(true);
        let armed = Armed {
            id: "reboot".into(),
            since: 0.0,
            deadline: 3.0,
        };
        assert!(matches!(press(&a, Some(&armed), 1.0), Press::Arm(_)));
    }

    #[test]
    fn capability_filtering_hides_na_and_no() {
        let caps = Capabilities {
            suspend: Capability::Yes,
            hibernate: Capability::Na,
            reboot: Capability::Challenge,
            power_off: Capability::No,
        };
        let config = Config::default();
        let mut problems = Vec::new();
        let actions = resolve(
            &config,
            &caps,
            Compositor::Hyprland,
            None,
            Some("hyprctl dispatch 'hl.dsp.exit()'"),
            "config.kdl",
            &mut problems,
        );
        let ids: Vec<_> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"suspend"));
        assert!(ids.contains(&"reboot"));
        assert!(!ids.contains(&"hibernate"));
        assert!(!ids.contains(&"shutdown"));
        assert!(ids.contains(&"logout"));
    }

    #[test]
    fn an_explicit_run_brings_back_a_capability_hidden_action() {
        let caps = Capabilities {
            hibernate: Capability::Na,
            ..Capabilities::all_yes()
        };
        let config = Config::from_source(
            "action \"hibernate\" { run \"systemctl hibernate\" }\n",
            "config.kdl",
            None,
        );
        let mut problems = Vec::new();
        let actions = resolve(
            &config,
            &caps,
            Compositor::Hyprland,
            None,
            Some("true"),
            "config.kdl",
            &mut problems,
        );
        assert!(actions.iter().any(|a| a.id == "hibernate"));
    }

    #[test]
    fn duplicate_keys_are_reported_and_cleared() {
        let config = Config::from_source(
            "action \"a\" { label \"A\"; run \"true\"; key \"x\" }\naction \"b\" { label \"B\"; run \"true\"; key \"X\" }\n",
            "config.kdl",
            None,
        );
        let mut problems = Vec::new();
        let actions = resolve(
            &config,
            &Capabilities::all_yes(),
            Compositor::Hyprland,
            None,
            Some("true"),
            "config.kdl",
            &mut problems,
        );
        let a = actions.iter().find(|a| a.id == "a").unwrap();
        let b = actions.iter().find(|a| a.id == "b").unwrap();
        assert_eq!(a.key, Some('x'));
        assert_eq!(b.key, None);
        assert!(problems.iter().any(|p| p.message.contains("already used")));
    }
}
