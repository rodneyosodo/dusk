//! The compositor's two jobs: which monitor is focused, and how to log out. Hyprland lives in
//! `hyprland.rs`; adding sway or niri means adding one file and one enum arm.

pub mod hyprland;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compositor {
    #[default]
    Unknown,
    Hyprland,
}

impl Compositor {
    /// Chosen from the session environment, never from a subprocess.
    pub fn detect() -> Self {
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
            || std::env::var("XDG_CURRENT_DESKTOP")
                .is_ok_and(|v| v.to_ascii_lowercase().contains("hyprland"))
        {
            Self::Hyprland
        } else {
            Self::Unknown
        }
    }

    pub fn supports_logout(self) -> bool {
        matches!(self, Self::Hyprland)
    }

    /// The default logout command, when the compositor has one.
    pub fn exit_command(self) -> Option<String> {
        match self {
            Self::Hyprland => Some(hyprland::exit_command()),
            Self::Unknown => None,
        }
    }

    /// The name of the focused monitor, for matching a Rune display.
    pub fn focused_monitor(self) -> Option<String> {
        match self {
            Self::Hyprland => hyprland::focused_monitor(),
            Self::Unknown => None,
        }
    }

    /// A short doctor line: compositor and version.
    pub fn describe(self) -> String {
        match self {
            Self::Hyprland => format!(
                "Hyprland {}",
                hyprland::version().unwrap_or_else(|| "?".into())
            ),
            Self::Unknown => "unknown (not Hyprland)".into(),
        }
    }
}
