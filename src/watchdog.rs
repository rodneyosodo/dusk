//! A menu that hangs must not sit over the screen. The event loop's poll beats a counter; a
//! watchdog thread exits the process if it stops beating, which removes every surface.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static BEAT: AtomicU64 = AtomicU64::new(0);
static STARTED: OnceLock<()> = OnceLock::new();

pub fn beat() {
    BEAT.fetch_add(1, Ordering::Relaxed);
}

pub fn start() {
    STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("dusk-watchdog".into())
            .spawn(|| {
                let started = Instant::now();
                let mut last = BEAT.load(Ordering::Relaxed);
                loop {
                    std::thread::sleep(Duration::from_secs(1));
                    let now = BEAT.load(Ordering::Relaxed);
                    if now == last && started.elapsed() > Duration::from_secs(10) {
                        eprintln!("dusk: the menu stopped responding; exiting so the surface does not hang");
                        std::process::exit(crate::EXIT_WATCHDOG);
                    }
                    last = now;
                }
            })
            .ok();
    });
}
