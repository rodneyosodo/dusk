//! Bundled line icons, drawn for dusk. Each is a small `Icon` body tinted by `.color()`.

use rune::Icon;

pub const LOCK: Icon = Icon {
    name: "dusk-lock",
    view: 24.0,
    body: r#"<rect x="5" y="10.5" width="14" height="10" rx="2.4" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M8 10.5V7.8a4 4 0 0 1 8 0v2.7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="12" cy="15.5" r="1.15" fill="currentColor"/>"#,
};

pub const LOGOUT: Icon = Icon {
    name: "dusk-logout",
    view: 24.0,
    body: r#"<path d="M14 4.5H6.8A1.8 1.8 0 0 0 5 6.3v11.4a1.8 1.8 0 0 0 1.8 1.8H14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M10.5 12h9M16.2 8.6 19.6 12l-3.4 3.4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub const SUSPEND: Icon = Icon {
    name: "dusk-suspend",
    view: 24.0,
    body: r#"<path d="M19 14.3A8 8 0 0 1 9.7 5a8.2 8.2 0 1 0 9.3 9.3Z" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>"#,
};

pub const HIBERNATE: Icon = Icon {
    name: "dusk-hibernate",
    view: 24.0,
    body: r#"<path d="M18.2 14.6A7.2 7.2 0 0 1 9.4 5.8a7.4 7.4 0 1 0 8.8 8.8Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M4.5 7.2h3l-3 3.4h3M2.8 12.9h2.3l-2.3 2.6h2.3" fill="none" stroke="currentColor" stroke-width="1.15" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub const REBOOT: Icon = Icon {
    name: "dusk-reboot",
    view: 24.0,
    body: r#"<path d="M19 12a7 7 0 1 1-2.2-5.1" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M18.5 3.8v4.4h-4.4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub const POWER: Icon = Icon {
    name: "dusk-power",
    view: 24.0,
    body: r#"<path d="M12 3.5v8" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/><path d="M7.2 6.2a7.3 7.3 0 1 0 9.6 0" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>"#,
};
