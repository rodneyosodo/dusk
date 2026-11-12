//! The header's time and date, from the C library so dusk needs no date crate. Headless runs use
//! a fixed time so snapshots stay identical between runs.

pub fn now() -> (String, String) {
    if rune::is_headless() {
        return ("21:47".into(), "Wednesday, 11 November".into());
    }
    // SAFETY: `time` with a null pointer only returns the current time; `localtime_r` writes into
    // our own zeroed `tm`; `strftime` writes at most `buf.len()` bytes into our buffer.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return (String::new(), String::new());
        }
        (format(&tm, c"%H:%M"), format(&tm, c"%A, %-d %B"))
    }
}

/// # Safety
/// `tm` must be a filled-in `struct tm`.
unsafe fn format(tm: &libc::tm, pattern: &std::ffi::CStr) -> String {
    let mut buf = [0u8; 64];
    // SAFETY: the caller passes a valid `tm`; the buffer length bounds the write.
    let n = unsafe { libc::strftime(buf.as_mut_ptr().cast(), buf.len(), pattern.as_ptr(), tm) };
    String::from_utf8_lossy(&buf[..n]).into_owned()
}
