//! `dusk doctor`: what the menu needs from the session, and what is missing.

use crate::compositor::Compositor;
use crate::system;

pub fn print() {
    let compositor = Compositor::detect();
    println!("dusk doctor");
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s == "wayland");
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| {
        if wayland {
            "wayland".into()
        } else {
            "unknown".into()
        }
    });
    println!(
        "  session      {session}, {}",
        if wayland { "Wayland" } else { "not Wayland" }
    );
    println!("  compositor   {}", compositor.describe());

    if compositor != Compositor::Unknown {
        if let Some(name) = compositor.focused_monitor() {
            println!("  monitor      focused: {name}");
        } else {
            println!(
                "  monitor      {} IPC did not answer; the first display is used",
                compositor.name()
            );
        }
        println!(
            "  layer-shell  {} provides it; the overlay namespace is \"dusk\"",
            compositor.name()
        );
    } else if wayland {
        println!("  layer-shell  assumed; dusk works where zwlr_layer_shell_v1 is available");
    } else {
        println!("  layer-shell  missing: dusk needs Wayland with layer-shell");
    }

    match compositor {
        Compositor::Hyprland => {
            if system::blur_rule_present() {
                println!("  blur rule    present for ^dusk");
            } else {
                println!("  blur rule    missing; add one line to your Hyprland config:");
                println!();
                println!("               {}", system::BLUR_SNIPPET_LUA);
                println!();
                println!("               (or, in hyprland.conf:)");
                for line in system::BLUR_SNIPPET_CONF.lines() {
                    println!("               {line}");
                }
            }
        }
        Compositor::Niri => {
            if system::niri_blur_rule_present() {
                println!("  blur rule    present for ^dusk");
            } else {
                println!("  blur rule    missing; add this to ~/.config/niri/config.kdl:");
                println!();
                for line in system::BLUR_SNIPPET_NIRI.lines() {
                    println!("               {line}");
                }
            }
        }
        Compositor::Sway | Compositor::Unknown => {
            println!("  blur rule    not provided by this compositor; the theme reads without it");
        }
    }

    let config = crate::config::Config::load(None);
    let lock_override = config
        .actions
        .iter()
        .find(|a| a.id == "lock")
        .and_then(|a| a.run.clone());
    let locker = lock_override.clone().or_else(|| system::locker(compositor));
    match &locker {
        Some(locker) if lock_override.is_some() => {
            println!("  locker       {locker} (from the lock action)");
        }
        Some(locker) => println!("  locker       {locker}"),
        None => {
            println!("  locker       none installed; Lock falls back to `loginctl lock-session`");
            println!("               that does nothing unless something handles lock-session");
        }
    }
    if system::has_command("notify-send") {
        println!("  notify       notify-send");
    } else {
        println!("  notify       no notify-send; failures only go to stderr");
    }

    let caps = system::capabilities();
    println!(
        "  logind       suspend {}, hibernate {}, reboot {}, poweroff {}",
        caps.suspend.label(),
        caps.hibernate.label(),
        caps.reboot.label(),
        caps.power_off.label()
    );
    if caps.suspend.allows() && locker.is_none() {
        println!("               warning: suspend has no locker; the machine would wake unlocked");
    }

    match system::inhibitors() {
        Ok(list) => {
            let blocking: Vec<_> = list.iter().filter(|i| i.mode == "block").collect();
            if blocking.is_empty() {
                println!("  inhibitors   none blocking");
            } else {
                for i in blocking {
                    println!("  inhibitor    {} ({})", i.describe(), i.what);
                }
            }
        }
        Err(err) => println!("  inhibitors   could not read: {err}"),
    }
}
