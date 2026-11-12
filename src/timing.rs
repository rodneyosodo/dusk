//! Opt-in phase timing for launch work. `DUSK_TIMING=1` prints to stderr.

use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

pub fn start() {
    let _ = START.set(Instant::now());
}

pub fn enabled() -> bool {
    std::env::var_os("DUSK_TIMING").is_some()
}

pub fn mark_once(label: &'static str) {
    static SEEN: OnceLock<std::sync::Mutex<std::collections::HashSet<&'static str>>> =
        OnceLock::new();
    let seen = SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    if seen.lock().unwrap().insert(label) {
        mark(label);
    }
}

pub fn mark(label: &str) {
    if !enabled() {
        return;
    }
    let ms = START.get().map_or(0, |start| start.elapsed().as_millis());
    eprintln!("dusk timing: {label} {ms} ms");
}
