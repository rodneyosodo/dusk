//! `dusk` command line: run, check, preview, doctor, init.

use std::path::PathBuf;

pub const HELP: &str = r#"dusk — a logout and power menu for Wayland compositors

USAGE
  dusk                 open, or close if already open
  dusk check [path]    report problems with line numbers; non-zero on failure
  dusk preview [path]  live-reloading preview; actions only print
  dusk doctor          layer-shell, blur rule, locker, logind capabilities
  dusk init            write a commented starter config; print the compositor setup snippet

OPTIONS
  --config PATH        use PATH instead of ~/.config/dusk/config.kdl
  --dry-run            the real menu, but actions only print
  -h, --help           show this help
  -V, --version        show the version

Rune's headless flags (`--snap`, `--scene`, `--tree`, `--script`, `--frames`, `--at`, `--size`)
pass through unchanged.
"#;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Run {
        config: Option<PathBuf>,
        dry_run: bool,
    },
    Check {
        path: Option<PathBuf>,
    },
    Preview {
        config: Option<PathBuf>,
    },
    Doctor,
    Init {
        force: bool,
    },
    Help,
    Version,
}

pub fn parse(args: &[String]) -> Command {
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let force = args.iter().any(|a| a == "--force");
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Command::Help;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        return Command::Version;
    }
    let config = args.iter().enumerate().find_map(|(i, arg)| {
        if arg == "--config" {
            args.get(i + 1).map(PathBuf::from)
        } else {
            arg.strip_prefix("--config=").map(PathBuf::from)
        }
    });
    let first = args.first().map(String::as_str);
    match first {
        Some("check") => Command::Check {
            path: args
                .get(1)
                .filter(|a| !a.starts_with('-'))
                .map(PathBuf::from)
                .or(config),
        },
        Some("preview") => Command::Preview {
            config: args
                .get(1)
                .filter(|a| !a.starts_with('-'))
                .map(PathBuf::from)
                .or(config),
        },
        Some("doctor") => Command::Doctor,
        Some("init") => Command::Init { force },
        Some("help") => Command::Help,
        Some("version") => Command::Version,
        _ => Command::Run { config, dry_run },
    }
}
