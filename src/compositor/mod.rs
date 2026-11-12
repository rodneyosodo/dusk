//! The compositor's two jobs: which monitor is focused, and how to log out. Each compositor
//! lives in its own file; adding another means one file and one enum arm.

pub mod hyprland;
pub mod niri;
pub mod sway;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compositor {
    #[default]
    Unknown,
    Hyprland,
    Sway,
    Niri,
}

impl Compositor {
    /// Chosen from the session environment, never from a subprocess.
    pub fn detect() -> Self {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        Self::detect_from(
            std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
            std::env::var_os("SWAYSOCK").is_some(),
            std::env::var_os("NIRI_SOCKET").is_some(),
            &desktop,
        )
    }

    /// The pure part of detection, so the priority order has a unit test.
    fn detect_from(hyprland: bool, sway: bool, niri: bool, desktop: &str) -> Self {
        let desktop = desktop.to_ascii_lowercase();
        if hyprland || desktop.contains("hyprland") {
            Self::Hyprland
        } else if sway || desktop.contains("sway") {
            Self::Sway
        } else if niri || desktop.contains("niri") {
            Self::Niri
        } else {
            Self::Unknown
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Hyprland => "Hyprland",
            Self::Sway => "Sway",
            Self::Niri => "Niri",
            Self::Unknown => "unknown compositor",
        }
    }

    pub fn supports_logout(self) -> bool {
        matches!(self, Self::Hyprland | Self::Sway | Self::Niri)
    }

    /// The default logout command, when the compositor has one.
    pub fn exit_command(self) -> Option<String> {
        match self {
            Self::Hyprland => Some(hyprland::exit_command()),
            Self::Sway => Some(sway::exit_command()),
            Self::Niri => Some(niri::exit_command()),
            Self::Unknown => None,
        }
    }

    /// The name of the focused monitor, for matching a Rune display.
    pub fn focused_monitor(self) -> Option<String> {
        match self {
            Self::Hyprland => hyprland::focused_monitor(),
            Self::Sway => sway::focused_monitor(),
            Self::Niri => niri::focused_monitor(),
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
            Self::Sway => format!("Sway {}", sway::version().unwrap_or_else(|| "?".into())),
            Self::Niri => format!("Niri {}", niri::version().unwrap_or_else(|| "?".into())),
            Self::Unknown => "unknown Wayland compositor".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_prefers_the_session_specific_socket() {
        assert_eq!(
            Compositor::detect_from(true, true, true, "sway"),
            Compositor::Hyprland
        );
        assert_eq!(
            Compositor::detect_from(false, true, true, ""),
            Compositor::Sway
        );
        assert_eq!(
            Compositor::detect_from(false, false, true, "Hyprland"),
            Compositor::Hyprland
        );
        assert_eq!(
            Compositor::detect_from(false, false, false, "niri"),
            Compositor::Niri
        );
        assert_eq!(
            Compositor::detect_from(false, false, false, "GNOME"),
            Compositor::Unknown
        );
    }

    #[test]
    fn sway_and_niri_have_a_logout_command() {
        assert!(Compositor::Sway.supports_logout());
        assert!(Compositor::Niri.supports_logout());
        assert!(Compositor::Sway.exit_command().is_some());
        assert!(Compositor::Niri.exit_command().is_some());
        assert!(Compositor::Unknown.exit_command().is_none());
    }
}
