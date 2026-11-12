//! dusk: a logout and power menu for Hyprland.

mod actions;
mod cli;
mod compositor;
mod config;
mod doctor;
mod instance;
mod model;
mod preview;
mod problem;
mod surfaces;
mod system;
mod theme;
mod timing;
mod ui;
mod watchdog;

use std::path::PathBuf;
use std::sync::Arc;

use actions::Mode;
use config::Config;
use problem::Problem;

pub const EXIT_CONFIG: i32 = 1;
pub const EXIT_RENDER: i32 = 3;
pub const EXIT_WATCHDOG: i32 = 70;

fn main() {
    // A closed pipe (for example `dusk doctor | head`) should end the process quietly.
    // SAFETY: restoring the default SIGPIPE disposition is process-wide and async-signal-safe.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    timing::start();
    rune::set_app_id("dusk");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = cli::parse(&args);
    let code = match command {
        cli::Command::Help => {
            print!("{}", cli::HELP);
            0
        }
        cli::Command::Version => {
            println!("dusk {}", env!("CARGO_PKG_VERSION"));
            0
        }
        cli::Command::Doctor => {
            doctor::print();
            0
        }
        cli::Command::Init { force } => match init_config(force) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("dusk: {err}");
                EXIT_CONFIG
            }
        },
        cli::Command::Check { path } => check(path),
        cli::Command::Preview { config } => {
            run(config, Mode::Preview);
            0
        }
        cli::Command::Run { config, dry_run } => {
            run(config, if dry_run { Mode::DryRun } else { Mode::Normal });
            0
        }
    };
    std::process::exit(code);
}

struct Prepared {
    config: Config,
    actions: Vec<actions::Action>,
    problems: Vec<Problem>,
    caps: system::Capabilities,
    compositor: compositor::Compositor,
    locker: Option<String>,
    logout: Option<String>,
}

fn prepare_with(path: Option<PathBuf>, explicit: bool) -> Prepared {
    let config = Config::load(path);
    let file = config
        .source
        .as_ref()
        .map_or_else(|| "<defaults>".to_owned(), |p| p.display().to_string());
    let mut problems = config.problems.clone();
    if explicit && config.source.as_ref().is_some_and(|p| !p.exists()) {
        problems.push(Problem::new(
            &file,
            1,
            "config file not found; using the built-in defaults",
        ));
    }
    let compositor = compositor::Compositor::detect();
    let caps = system::capabilities();
    let locker = system::locker(compositor);
    let logout = system::logout_command(compositor);
    let resolved = actions::resolve(
        &config,
        &caps,
        compositor,
        locker.as_deref(),
        logout.as_deref(),
        &file,
        &mut problems,
    );
    Prepared {
        config,
        actions: resolved,
        problems,
        caps,
        compositor,
        locker,
        logout,
    }
}

fn print_problems(problems: &[Problem]) {
    for problem in problems {
        eprintln!("dusk: {problem}");
    }
}

fn check(path: Option<PathBuf>) -> i32 {
    let explicit = path.is_some();
    let prepared = prepare_with(path.or_else(Config::default_path), explicit);
    print_problems(&prepared.problems);
    if prepared.problems.is_empty() {
        println!(
            "no problems: theme {}, layout {:?}, {} action(s)",
            prepared.config.theme.id,
            prepared.config.effective_layout(),
            prepared.actions.len()
        );
        0
    } else {
        eprintln!(
            "dusk: {} problem(s); the menu still opens with the settings that parsed",
            prepared.problems.len()
        );
        EXIT_CONFIG
    }
}

fn run(config_path: Option<PathBuf>, mode: Mode) {
    let explicit = config_path.is_some();
    let path = config_path.or_else(Config::default_path);
    let mut instance = None;
    if mode != Mode::Preview && rune::tooling::headless_run().is_none() && !rune::is_headless() {
        match instance::acquire() {
            Ok(instance::Acquired::First(first)) => instance = Some(Arc::new(first)),
            Ok(instance::Acquired::Toggle) => return,
            Err(err) => {
                eprintln!("dusk: single-instance lock failed: {err}");
            }
        }
    }

    rune::prewarm_gpu();
    let prepared = prepare_with(path.clone(), explicit);
    print_problems(&prepared.problems);
    if mode == Mode::Preview
        && let Some(path) = &path
        && !path.exists()
    {
        eprintln!(
            "dusk: no config at {}; `dusk init` writes a starter",
            path.display()
        );
    }

    timing::mark("prepared");
    let launch = surfaces::Launch {
        config: Arc::new(prepared.config),
        actions: prepared.actions,
        problem: prepared.problems.first().cloned(),
        mode,
        path,
        caps: prepared.caps,
        compositor: prepared.compositor,
        locker: prepared.locker,
        logout: prepared.logout,
        instance,
    };
    if let Err(error) = surfaces::run(launch) {
        eprintln!("dusk: {error}");
        std::process::exit(EXIT_RENDER);
    }
}

fn init_config(force: bool) -> Result<(), String> {
    let path = Config::default_path().ok_or_else(|| "cannot find a config directory".to_owned())?;
    if path.exists() && !force {
        println!(
            "{} already exists; leaving it alone (--force overwrites)",
            path.display()
        );
    } else {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        std::fs::write(&path, STARTER)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        println!("wrote {}", path.display());
    }
    println!();
    match compositor::Compositor::detect() {
        compositor::Compositor::Hyprland => {
            println!("Add this to your Hyprland config so the desktop blurs behind dusk:");
            println!();
            println!("{}", system::BLUR_SNIPPET_LUA);
            println!();
            println!("Or, in an older hyprland.conf:");
            for line in system::BLUR_SNIPPET_CONF.lines() {
                println!("{line}");
            }
        }
        compositor::Compositor::Niri => {
            println!("Add this to ~/.config/niri/config.kdl so the desktop blurs behind dusk:");
            println!();
            for line in system::BLUR_SNIPPET_NIRI.lines() {
                println!("{line}");
            }
        }
        compositor::Compositor::Sway => {
            println!("Add a keybind to ~/.config/sway/config:");
            println!();
            println!("bindsym $mod+Escape exec dusk");
            println!();
            println!("Sway has no built-in layer blur; dusk's veil is denser there.");
        }
        compositor::Compositor::Unknown => {
            println!("No compositor-specific blur rule is needed; dusk uses its own veil.");
        }
    }
    Ok(())
}

const STARTER: &str = r##"// ~/.config/dusk/config.kdl
//
// No config is needed: dusk looks good with the built-in defaults. Uncomment or copy a line.

theme "dusk"          // dusk, halo, mono, or a path to a theme.kdl
// layout "row"       // row, grid, list
// appearance "auto"  // auto, light, dark

// tokens {
//     accent "#d38be0"
//     radius 28
// }
// tokens "light" {
//     dim "white/40"
// }

// hide "hibernate"
// order "lock" "logout" "suspend" "reboot" "shutdown"

// Change one field of a built-in; the rest keep their defaults.
// action "logout" { run "hyprshutdown"; key "e" }

// Add your own.
// action "firmware" {
//     label "Firmware"
//     icon "~/.config/dusk/icons/chip.svg"
//     run "systemctl reboot --firmware-setup"
//     key "f"
//     confirm #true
// }
"##;
