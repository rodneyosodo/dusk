//! `dusk preview`: watch the config and theme mtimes and swap the live menu's look without
//! restarting. Actions only print.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::actions;
use crate::config::Config;
use crate::model::{self, Context};

pub fn start(launch: crate::surfaces::Launch) {
    let Some(path) = launch.path.clone().or_else(|| launch.config.source.clone()) else {
        eprintln!("dusk: preview has no config file to watch");
        return;
    };
    std::thread::Builder::new()
        .name("dusk-preview".into())
        .spawn(move || watch(path, launch))
        .ok();
}

fn watch(path: PathBuf, launch: crate::surfaces::Launch) {
    let beside = path.parent().map(Path::to_path_buf);
    let mut last = mtime(&path);
    let mut theme_path = watch_theme(&launch.config, beside.as_deref());
    let mut theme_last = theme_path.as_deref().and_then(mtime);
    loop {
        std::thread::sleep(Duration::from_millis(150));
        let now = mtime(&path);
        let now_theme = theme_path.as_deref().and_then(mtime);
        if now == last && now_theme == theme_last {
            continue;
        }
        // Remember what was seen before loading: an edit made during the load must trigger another
        // pass instead of being treated as already loaded.
        last = now;
        let seen_theme = now_theme;
        let mut config = Config::load(Some(path.clone()));
        let file = path.display().to_string();
        let mut problems = std::mem::take(&mut config.problems);
        let actions = actions::resolve(
            &config,
            &launch.caps,
            launch.compositor,
            launch.locker.as_deref(),
            launch.logout.as_deref(),
            &file,
            &mut problems,
        );
        let problem = problems.first().cloned();
        for p in &problems {
            eprintln!("dusk: {p}");
        }
        // The next edit may be in a different theme file, or the theme may have been removed.
        let next_theme = watch_theme(&config, beside.as_deref());
        theme_last = if next_theme == theme_path {
            seen_theme
        } else {
            None
        };
        theme_path = next_theme;
        let config = Arc::new(config);
        model::set_context(Context {
            config: config.clone(),
            actions: actions.clone(),
            problem: problem.clone(),
            mode: launch.mode,
            blur: crate::system::blur_rule_present(),
        });
        if let Some(handles) = model::handles() {
            handles.replace(config, actions, problem);
        }
        eprintln!("dusk: preview reloaded {}", path.display());
    }
}

/// The theme file this config would reload from, including a selected file that is currently
/// missing. Built-ins are embedded and return `None`.
fn watch_theme(config: &Config, beside: Option<&Path>) -> Option<PathBuf> {
    config
        .theme
        .source
        .clone()
        .or_else(|| crate::theme::theme_candidate_path(&config.theme.id, beside))
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}
