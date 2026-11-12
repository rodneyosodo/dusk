//! System services dusk asks about: logind capabilities and inhibitors, the locker, and the
//! small `which` checks used by built-in actions and `doctor`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use zbus::Proxy;

use crate::compositor::Compositor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Capability {
    Yes,
    /// A password or polkit prompt is needed; the action is still shown.
    Challenge,
    No,
    /// The machine cannot do it at all.
    Na,
    /// logind's `inhibited`: an inhibitor is blocking it, but the action is still offered so
    /// the menu can name the blocker when it is pressed.
    Inhibited,
    #[default]
    Unknown,
}

impl Capability {
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "yes" => Self::Yes,
            "challenge" => Self::Challenge,
            "no" => Self::No,
            "na" => Self::Na,
            "inhibited" => Self::Inhibited,
            _ => Self::Unknown,
        }
    }

    pub fn allows(self) -> bool {
        matches!(self, Self::Yes | Self::Challenge | Self::Inhibited)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::Challenge => "challenge",
            Self::No => "no",
            Self::Na => "n/a",
            Self::Inhibited => "inhibited",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub suspend: Capability,
    pub hibernate: Capability,
    pub reboot: Capability,
    pub power_off: Capability,
}

impl Capabilities {
    #[cfg(test)]
    pub fn all_yes() -> Self {
        Self {
            suspend: Capability::Yes,
            hibernate: Capability::Yes,
            reboot: Capability::Yes,
            power_off: Capability::Yes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inhibitor {
    pub what: String,
    pub who: String,
    pub why: String,
    pub mode: String,
    pub uid: u32,
}

impl Inhibitor {
    pub fn blocks(&self, what: &str) -> bool {
        self.mode == "block"
            && self
                .what
                .split(':')
                .any(|w| w == what || (what == "shutdown" && w == "shutdown"))
    }

    pub fn describe(&self) -> String {
        if self.why.trim().is_empty() {
            self.who.clone()
        } else {
            format!("{} ({})", self.why, self.who)
        }
    }
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

async fn manager() -> Result<Proxy<'static>, zbus::Error> {
    let conn = zbus::Connection::system().await?;
    Proxy::new(
        &conn,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
}

/// Reads logind's four `Can*` answers. A missing logind or D-Bus leaves the answers Unknown,
/// which hides the power actions and is explained by `doctor`.
pub fn capabilities() -> Capabilities {
    runtime().block_on(async {
        let proxy = match manager().await {
            Ok(proxy) => proxy,
            Err(_) => return Capabilities::default(),
        };
        let mut caps = Capabilities::default();
        for (slot, method) in [
            (&mut caps.suspend, "CanSuspend"),
            (&mut caps.hibernate, "CanHibernate"),
            (&mut caps.reboot, "CanReboot"),
            (&mut caps.power_off, "CanPowerOff"),
        ] {
            if let Ok(answer) = proxy.call::<_, _, String>(method, &()).await {
                *slot = Capability::parse(&answer);
            }
        }
        caps
    })
}

pub fn inhibitors() -> Result<Vec<Inhibitor>, String> {
    runtime().block_on(async {
        let proxy = manager().await.map_err(|e| e.to_string())?;
        let rows: Vec<(String, String, String, String, u32, u32)> = proxy
            .call("ListInhibitors", &())
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(what, who, why, mode, uid, _pid)| Inhibitor {
                what,
                who,
                why,
                mode,
                uid,
            })
            .collect())
    })
}

/// Watch logind for the session locking and the machine preparing to sleep. Focus loss covers
/// most overlays, but a lock screen or suspend can keep the compositor's keyboard focus; this
/// closes the menu before it can sit over a locked or resumed screen.
///
/// The watcher owns its runtime: `inhibitors()` and `blocker()` use the shared single-thread
/// runtime, and a long-lived `block_on` on that runtime would deadlock them during a power action.
pub fn watch_session(close: std::sync::Arc<dyn Fn() + Send + Sync>) {
    use std::sync::mpsc;
    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("dusk-logind".into())
        .spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                let _ = ready_tx.send(());
                return;
            };
            let mut announced = false;
            loop {
                let result = runtime.block_on(async {
                    let mut ready = || {
                        if !announced {
                            let _ = ready_tx.send(());
                            announced = true;
                        }
                    };
                    watch_session_once(&close, &mut ready).await
                });
                if !announced {
                    let _ = ready_tx.send(());
                    announced = true;
                }
                match result {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(error) if std::env::var_os("DUSK_DEBUG").is_some() => {
                        eprintln!("dusk: session watch unavailable: {error}");
                    }
                    Err(_) => {}
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        })
        .ok();
    // Give the watcher a moment to install its subscriptions before the menu is shown. If
    // logind is slow or absent, the menu still opens and the watcher keeps retrying.
    let _ = ready_rx.recv_timeout(std::time::Duration::from_millis(300));
}

async fn watch_session_once(
    close: &std::sync::Arc<dyn Fn() + Send + Sync>,
    ready: &mut dyn FnMut(),
) -> Result<bool, zbus::Error> {
    let conn = zbus::Connection::system().await?;
    watch_connection(&conn, close, ready).await
}

/// The signal half of the watcher, split from the system-bus connection so tests can drive it
/// over a private bus with a mock logind.
async fn watch_connection(
    conn: &zbus::Connection,
    close: &std::sync::Arc<dyn Fn() + Send + Sync>,
    ready: &mut dyn FnMut(),
) -> Result<bool, zbus::Error> {
    use futures_util::StreamExt;
    let manager = zbus::Proxy::new(
        conn,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    let mut sleep = manager.receive_signal("PrepareForSleep").await?;
    let path = session_path(&manager).await;
    let mut lock = match path {
        Some(path) => {
            let session = zbus::Proxy::new(
                conn,
                "org.freedesktop.login1",
                path.as_str(),
                "org.freedesktop.login1.Session",
            )
            .await?;
            let stream = session.receive_signal("Lock").await?;
            // A lock between resolving the path and subscribing is not replayed; the property
            // catches that window.
            let locked: bool = session.get_property("LockedHint").await.unwrap_or(false);
            if locked {
                ready();
                close();
                return Ok(true);
            }
            Some(stream)
        }
        None => None,
    };
    ready();
    if let Some(lock) = lock.as_mut() {
        loop {
            tokio::select! {
                Some(message) = sleep.next() => {
                    if message.body().deserialize::<bool>().unwrap_or(false) {
                        close();
                        return Ok(true);
                    }
                }
                Some(_) = lock.next() => {
                    close();
                    return Ok(true);
                }
                else => return Ok(false),
            }
        }
    }
    while let Some(message) = sleep.next().await {
        if message.body().deserialize::<bool>().unwrap_or(false) {
            close();
            return Ok(true);
        }
    }
    Ok(false)
}

async fn session_path(manager: &zbus::Proxy<'_>) -> Option<zbus::OwnedObjectPath> {
    if let Ok(id) = std::env::var("XDG_SESSION_ID")
        && let Ok(path) = manager.call("GetSession", &id).await
    {
        return Some(path);
    }
    manager
        .call("GetSessionByPID", &std::process::id())
        .await
        .ok()
}

/// The first inhibitor that blocks `what` (`shutdown` for power off as well as reboot).
pub fn blocker(what: &str) -> Option<Inhibitor> {
    inhibitors().ok()?.into_iter().find(|i| i.blocks(what))
}

/// A command on PATH.
pub fn has_command(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let p = dir.join(name);
        p.is_file() && is_executable(&p)
    })
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

/// A desktop notification when available, otherwise only stderr.
pub fn notify(summary: &str, body: &str) {
    eprintln!("dusk: {summary}: {body}");
    let _ = std::process::Command::new("notify-send")
        .arg("dusk")
        .arg(summary)
        .arg(body)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn find_command(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find(|name| has_command(name))
        .map(|name| (*name).to_owned())
}

pub fn locker(compositor: Compositor) -> Option<String> {
    // hyprlock speaks Hyprland's IPC; the wlroots lockers work on Sway and Niri.
    match compositor {
        Compositor::Hyprland => {
            find_command(&["hyprlock", "swaylock", "swaylock-effects", "gtklock"])
        }
        _ => find_command(&["swaylock", "swaylock-effects", "gtklock", "hyprlock"]),
    }
}

/// The default logout command: `hyprshutdown` when installed, else the compositor's own exit.
pub fn logout_command(compositor: Compositor) -> Option<String> {
    if !compositor.supports_logout() {
        return None;
    }
    if compositor == Compositor::Hyprland && has_command("hyprshutdown") {
        return Some("hyprshutdown".into());
    }
    compositor.exit_command()
}

pub fn hyprland_config_files() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let dir = PathBuf::from(home).join(".config/hypr");
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|e| e == "lua" || e == "conf") {
            files.push(path);
        }
    }
}

/// Whether a Hyprland config mentions a blur rule for the dusk namespace.
pub fn blur_rule_present() -> bool {
    for file in hyprland_config_files() {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with('#') {
                continue;
            }
            let matches_rule = l.contains("layer_rule") || l.contains("layerrule");
            if matches_rule
                && l.contains("dusk")
                && (l.contains("blur") || text.contains("blur = true"))
            {
                return true;
            }
        }
    }
    false
}

pub const BLUR_SNIPPET_LUA: &str = r#"hl.layer_rule({ match = { namespace = "^dusk" }, blur = true, ignore_alpha = 0.02, no_anim = true })"#;
pub const BLUR_SNIPPET_CONF: &str = r#"layerrule = blur, ^dusk
layerrule = ignorealpha 0.02, ^dusk
layerrule = noanim, ^dusk"#;

pub const BLUR_SNIPPET_NIRI: &str = r#"layer-rule {
    match namespace="^dusk$"
    background-effect {
        blur true
    }
}"#;

/// Niri config files: `$XDG_CONFIG_HOME/niri`, or `~/.config/niri`, including `include`d KDL.
pub fn niri_config_files() -> Vec<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let Some(base) = base else {
        return Vec::new();
    };
    let dir = base.join("niri");
    let mut files = Vec::new();
    collect_kdl(&dir, &mut files);
    files
}

fn collect_kdl(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_kdl(&path, files);
        } else if path.extension().is_some_and(|e| e == "kdl") {
            files.push(path);
        }
    }
}

/// Whether a niri config has a layer rule with `dusk` and `blur` in the same block.
pub fn niri_blur_rule_present() -> bool {
    for file in niri_config_files() {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !line.contains("layer-rule") {
                continue;
            }
            let block = lines[i..lines.len().min(i + 12)].join(" ");
            if block.contains("dusk") && block.contains("blur") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_answers_allow_yes_and_challenge_only() {
        assert!(Capability::parse("yes").allows());
        assert!(Capability::parse("challenge").allows());
        assert!(!Capability::parse("no").allows());
        assert!(!Capability::parse("na").allows());
        assert!(Capability::parse("inhibited").allows());
        assert_eq!(Capability::parse("weird"), Capability::Unknown);
    }

    #[test]
    fn a_block_inhibitor_for_shutdown_blocks_power_off() {
        let i = Inhibitor {
            what: "shutdown:sleep".into(),
            who: "browser".into(),
            why: "Saving state".into(),
            mode: "block".into(),
            uid: 1000,
        };
        assert!(i.blocks("shutdown"));
        assert!(i.blocks("sleep"));
        assert_eq!(i.describe(), "Saving state (browser)");
        let delay = Inhibitor {
            mode: "delay".into(),
            ..i
        };
        assert!(!delay.blocks("shutdown"));
    }
    // --- mock logind -------------------------------------------------------------

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    const MANAGER_PATH: &str = "/org/freedesktop/login1";
    const SESSION_PATH: &str = "/org/freedesktop/login1/session/test";

    struct Login1;

    #[zbus::interface(name = "org.freedesktop.login1.Manager")]
    impl Login1 {
        async fn get_session(&self, _id: String) -> zbus::OwnedObjectPath {
            session_object()
        }

        #[zbus(name = "GetSessionByPID")]
        async fn get_session_by_pid(&self, _pid: u32) -> zbus::OwnedObjectPath {
            session_object()
        }

        #[zbus(signal)]
        async fn prepare_for_sleep(
            emitter: &zbus::object_server::SignalEmitter<'_>,
            start: bool,
        ) -> zbus::Result<()>;
    }

    fn session_object() -> zbus::OwnedObjectPath {
        zbus::OwnedObjectPath::try_from(SESSION_PATH).expect("valid session path")
    }

    struct Session {
        locked: Arc<AtomicBool>,
    }

    #[zbus::interface(name = "org.freedesktop.login1.Session")]
    impl Session {
        #[zbus(property)]
        async fn locked_hint(&self) -> bool {
            self.locked.load(Ordering::SeqCst)
        }

        #[zbus(signal)]
        async fn lock(emitter: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;
    }

    struct PrivateBus {
        child: std::process::Child,
        address: String,
    }

    impl Drop for PrivateBus {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn private_bus() -> Option<PrivateBus> {
        use std::io::BufRead;
        let mut child = match std::process::Command::new("dbus-daemon")
            .args(["--session", "--print-address", "--nofork"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping: dbus-daemon is not installed");
                return None;
            }
            Err(error) => panic!("could not start dbus-daemon: {error}"),
        };
        let stdout = child.stdout.take().expect("dbus-daemon stdout");
        let mut line = String::new();
        std::io::BufReader::new(stdout)
            .read_line(&mut line)
            .expect("read the private bus address");
        let address = line.trim().to_owned();
        assert!(!address.is_empty(), "dbus-daemon printed no address");
        Some(PrivateBus { child, address })
    }

    fn logind_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    }

    async fn mock_logind(
        bus: &PrivateBus,
    ) -> (zbus::Connection, zbus::Connection, Arc<AtomicBool>) {
        let server = zbus::connection::Builder::address(bus.address.as_str())
            .build()
            .await
            .expect("server connection");
        let locked = Arc::new(AtomicBool::new(false));
        server
            .object_server()
            .at(MANAGER_PATH, Login1)
            .await
            .expect("manager object");
        server
            .object_server()
            .at(
                SESSION_PATH,
                Session {
                    locked: locked.clone(),
                },
            )
            .await
            .expect("session object");
        server
            .request_name("org.freedesktop.login1")
            .await
            .expect("own logind name");
        let client = zbus::connection::Builder::address(bus.address.as_str())
            .build()
            .await
            .expect("client connection");
        (server, client, locked)
    }

    fn close_flag() -> (Arc<dyn Fn() + Send + Sync>, Arc<AtomicBool>) {
        let closed = Arc::new(AtomicBool::new(false));
        let flag = closed.clone();
        let close: Arc<dyn Fn() + Send + Sync> =
            Arc::new(move || flag.store(true, Ordering::SeqCst));
        (close, closed)
    }

    fn sleep_emitter(server: &zbus::Connection) -> zbus::object_server::SignalEmitter<'_> {
        zbus::object_server::SignalEmitter::new(server, MANAGER_PATH).expect("manager emitter")
    }

    #[test]
    fn prepare_for_sleep_closes_only_when_sleeping() {
        let Some(bus) = private_bus() else { return };
        logind_runtime().block_on(async {
            let (server, client, _) = mock_logind(&bus).await;
            let (close, closed) = close_flag();
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
            let mut ready_tx = Some(ready_tx);
            let mut on_ready = || {
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(());
                }
            };
            let watch = watch_connection(&client, &close, &mut on_ready);
            tokio::pin!(watch);
            tokio::select! {
                _ = ready_rx => {}
                result = &mut watch => panic!("watcher stopped before it was ready: {result:?}"),
            }
            let emitter = sleep_emitter(&server);
            Login1::prepare_for_sleep(&emitter, false)
                .await
                .expect("emit resume");
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(250), &mut watch)
                    .await
                    .is_err(),
                "resuming must not close the menu"
            );
            assert!(
                !closed.load(Ordering::SeqCst),
                "resuming must not call close"
            );
            Login1::prepare_for_sleep(&emitter, true)
                .await
                .expect("emit sleep");
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), &mut watch)
                .await
                .expect("sleep did not close the menu")
                .expect("watch error");
            assert!(result);
            assert!(closed.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn a_lock_signal_closes_the_menu() {
        let Some(bus) = private_bus() else { return };
        logind_runtime().block_on(async {
            let (server, client, _) = mock_logind(&bus).await;
            let (close, closed) = close_flag();
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
            let mut ready_tx = Some(ready_tx);
            let mut on_ready = || {
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(());
                }
            };
            let watch = watch_connection(&client, &close, &mut on_ready);
            tokio::pin!(watch);
            tokio::select! {
                _ = ready_rx => {}
                result = &mut watch => panic!("watcher stopped before it was ready: {result:?}"),
            }
            let emitter = zbus::object_server::SignalEmitter::new(&server, SESSION_PATH)
                .expect("session emitter");
            Session::lock(&emitter).await.expect("emit lock");
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), &mut watch)
                .await
                .expect("lock did not close the menu")
                .expect("watch error");
            assert!(result);
            assert!(closed.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn a_lock_before_subscribing_closes_via_the_hint() {
        let Some(bus) = private_bus() else { return };
        logind_runtime().block_on(async {
            let (_server, client, locked) = mock_logind(&bus).await;
            locked.store(true, Ordering::SeqCst);
            let (close, closed) = close_flag();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                watch_connection(&client, &close, &mut || {}),
            )
            .await
            .expect("locked hint did not close the menu")
            .expect("watch error");
            assert!(result);
            assert!(closed.load(Ordering::SeqCst));
        });
    }
}
