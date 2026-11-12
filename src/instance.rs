//! Single instance and toggle. An `flock` under `$XDG_RUNTIME_DIR` keeps one menu; a Unix socket
//! lets a second launch ask the first to close. If it does not leave in time, it is killed.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Instance {
    _lock: File,
    listener: UnixListener,
    socket: PathBuf,
}

pub enum Acquired {
    First(Instance),
    Toggle,
}

pub fn acquire() -> std::io::Result<Acquired> {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir)?;
    let lock_path = dir.join("dusk.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    // SAFETY: flock only operates on the valid fd we just opened.
    let locked = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    if !locked {
        toggle_socket(&dir);
        return Ok(Acquired::Toggle);
    }
    let _ = lock.set_len(0);
    let mut lock = lock;
    let _ = writeln!(lock, "{}", std::process::id());

    let socket = dir.join("dusk.sock");
    let listener = bind_socket(&socket).map_err(|e| {
        // A listener is required for toggle; leaving the lock held would wedge the next launch.
        std::io::Error::new(e.kind(), format!("{e}: {}", socket.display()))
    })?;
    Ok(Acquired::First(Instance {
        _lock: lock,
        listener,
        socket,
    }))
}

fn bind_socket(path: &PathBuf) -> std::io::Result<UnixListener> {
    match UnixListener::bind(path) {
        Ok(listener) => Ok(listener),
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            // A stale socket from a killed process: remove it once.
            let _ = std::fs::remove_file(path);
            UnixListener::bind(path)
        }
        Err(err) => Err(err),
    }
}

impl Instance {
    /// Serve toggle requests on a thread. `close` runs on the app's thread.
    pub fn serve(&self, close: Arc<dyn Fn() + Send + Sync>) {
        let listener = self.listener.try_clone();
        let Ok(listener) = listener else { return };
        std::thread::Builder::new()
            .name("dusk-instance".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let mut line = String::new();
                    let _ = BufReader::new(stream).read_line(&mut line);
                    if line.trim() == "toggle" {
                        eprintln!("dusk: toggle received");
                        close();
                    }
                }
            })
            .ok();
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Ask the first instance to close; wait, then force it if its event loop is wedged.
fn toggle_socket(dir: &Path) {
    let socket = dir.join("dusk.sock");
    if let Ok(mut stream) = UnixStream::connect(&socket) {
        let _ = stream.write_all(b"toggle\n");
        let _ = stream.flush();
        let _ = stream.shutdown(std::net::Shutdown::Write);
    }
    let Some(pid) = read_pid(dir) else {
        eprintln!("dusk: no pid for toggle");
        return;
    };
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !alive(pid) {
            return;
        }
    }
    signal(pid, libc::SIGTERM);
    std::thread::sleep(std::time::Duration::from_millis(200));
    if alive(pid) {
        signal(pid, libc::SIGKILL);
    }
}

fn read_pid(dir: &Path) -> Option<i32> {
    let text = std::fs::read_to_string(dir.join("dusk.lock")).ok()?;
    text.trim().parse().ok()
}

fn alive(pid: i32) -> bool {
    // SAFETY: kill with signal 0 only checks whether the pid exists.
    unsafe { libc::kill(pid, 0) == 0 }
}

fn signal(pid: i32, signal: i32) {
    // SAFETY: a signal to the pid recorded in our own runtime directory.
    unsafe {
        libc::kill(pid, signal);
    }
}

fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/dusk-{}", unsafe { libc::getuid() })))
}
