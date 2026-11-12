//! `dusk preview`: watch the config's mtime and swap the live menu's look without restarting.
//! Actions only print.

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
    let mut last = mtime(&path);
    let mut theme_path = launch.config.theme.source.clone();
    let mut theme_last = theme_path.as_deref().and_then(mtime);
    loop {
        std::thread::sleep(Duration::from_millis(150));
        let config_changed = mtime(&path) != last;
        let theme_changed = theme_path.as_deref().and_then(mtime) != theme_last;
        if !config_changed && !theme_changed {
            continue;
        }
        last = mtime(&path);
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
        theme_path = config.theme.source.clone();
        theme_last = theme_path.as_deref().and_then(mtime);
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

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}
