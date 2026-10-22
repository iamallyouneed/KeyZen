use std::path::Path;

pub const MENU_LABELS: [&str; 4] = ["Pause", "Start at login", "Choose key config...", "Quit"];

const ID_PAUSE: usize = 1001;
const ID_START_AT_LOGIN: usize = 1002;
const ID_CHOOSE_KEY_CONFIG: usize = 1003;
const ID_QUIT: usize = 1004;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMenuCommand {
    Pause,
    StartAtLogin,
    ChooseKeyConfig,
    Quit,
}

pub fn command_from_menu_id(id: usize) -> Option<TrayMenuCommand> {
    match id {
        ID_PAUSE => Some(TrayMenuCommand::Pause),
        ID_START_AT_LOGIN => Some(TrayMenuCommand::StartAtLogin),
        ID_CHOOSE_KEY_CONFIG => Some(TrayMenuCommand::ChooseKeyConfig),
        ID_QUIT => Some(TrayMenuCommand::Quit),
        _ => None,
    }
}

/// One-line summary of the active key config shown at the top of the tray menu.
pub(crate) fn status_label(loaded: bool, path: &Path, mapped_keys: usize) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    if !loaded {
        format!("Config: {name} (not loaded)")
    } else if mapped_keys == 0 {
        format!("Config: {name} (no mappings)")
    } else {
        format!("Config: {name} ({mapped_keys} keys)")
    }
}

/// A session ends cleanly when the tray asked for shutdown (Quit, Windows session end,
/// or a close on the hidden window) rather than the message loop failing on its own.
pub fn is_clean_shutdown_reason(reason: &str) -> bool {
    matches!(
        reason,
        "tray_menu_quit"
            | "windows_session_ending"
            | "hidden_window_close_message"
            | "hidden_window_destroyed"
    )
}

#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("KeyZen tray is only supported on Windows")]
    UnsupportedPlatform,
    #[error("another KeyZen remapper instance is already running")]
    AlreadyRunning,
    #[error("{0}")]
    Operation(String),
}

#[cfg(windows)]
mod platform;

#[cfg(not(windows))]
pub fn run_tray_app() -> Result<(), TrayError> {
    Err(TrayError::UnsupportedPlatform)
}

#[cfg(windows)]
pub use platform::run_tray_app;

#[cfg(test)]
mod tests {
    use super::{
        MENU_LABELS, TrayMenuCommand, command_from_menu_id, is_clean_shutdown_reason, status_label,
    };
    use std::path::Path;

    #[test]
    fn describes_the_active_config() {
        let path = Path::new(r"C:\Users\me\KeyZen\mykeyzen.yaml");

        assert_eq!(
            status_label(true, path, 20),
            "Config: mykeyzen.yaml (20 keys)"
        );
        assert_eq!(
            status_label(true, path, 0),
            "Config: mykeyzen.yaml (no mappings)"
        );
        assert_eq!(
            status_label(false, path, 0),
            "Config: mykeyzen.yaml (not loaded)"
        );
    }

    #[test]
    fn maps_command_ids_to_english_menu_events() {
        assert_eq!(
            MENU_LABELS,
            ["Pause", "Start at login", "Choose key config...", "Quit"]
        );
        assert_eq!(command_from_menu_id(1001), Some(TrayMenuCommand::Pause));
        assert_eq!(
            command_from_menu_id(1002),
            Some(TrayMenuCommand::StartAtLogin)
        );
        assert_eq!(
            command_from_menu_id(1003),
            Some(TrayMenuCommand::ChooseKeyConfig)
        );
        assert_eq!(command_from_menu_id(1004), Some(TrayMenuCommand::Quit));
        assert_eq!(command_from_menu_id(9999), None);
    }

    #[test]
    fn treats_shutdown_requests_as_clean_session_endings() {
        for reason in [
            "tray_menu_quit",
            "windows_session_ending",
            "hidden_window_close_message",
            "hidden_window_destroyed",
        ] {
            assert!(is_clean_shutdown_reason(reason), "{reason}");
        }

        for reason in [
            "message_loop_ended_without_destroy",
            "tray_menu_quit_later",
            "",
        ] {
            assert!(!is_clean_shutdown_reason(reason), "{reason}");
        }
    }
}
