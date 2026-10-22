use super::super::{ID_CHOOSE_KEY_CONFIG, ID_PAUSE, ID_QUIT, ID_START_AT_LOGIN};
use super::{STATE, icon, operation_error, window};
use crate::RuntimeCommand;
use crate::app_settings;
use crate::startup::{TaskSchedulerStartup, update_start_at_login};
use crate::tray::{ShutdownReason, TrayError, TrayMenuCommand, status_label};
use std::ffi::c_void;
use std::path::PathBuf;
use windows::Win32::Foundation::{ERROR_CANCELLED, HWND, POINT};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, MENU_ITEM_FLAGS, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SetForegroundWindow, TPM_RIGHTBUTTON,
    TrackPopupMenu,
};
use windows::core::{HRESULT, PCWSTR, w};

pub(super) fn show_context_menu(hwnd: HWND) -> Result<(), TrayError> {
    let (paused, start_at_login, loaded, config_path, mapped_keys) = STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| {
            (
                state.paused,
                state.settings.start_at_login,
                state.has_config,
                state.config_path.clone(),
                state.mapped_keys,
            )
        })
        .unwrap_or((true, false, false, PathBuf::new(), 0));
    let status = status_label(loaded, &config_path, mapped_keys);
    let status_wide = status.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let menu = unsafe { CreatePopupMenu() }
        .map_err(|error| operation_error(format!("CreatePopupMenu failed: {error}")))?;

    unsafe {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(status_wide.as_ptr()))
            .map_err(operation_error)?;
        AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()).map_err(operation_error)?;
        AppendMenuW(
            menu,
            MF_STRING | checked_flag(paused),
            ID_PAUSE,
            w!("Pause"),
        )
        .map_err(operation_error)?;
        AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()).map_err(operation_error)?;
        AppendMenuW(
            menu,
            MF_STRING | checked_flag(start_at_login),
            ID_START_AT_LOGIN,
            w!("Start at login"),
        )
        .map_err(operation_error)?;
        AppendMenuW(
            menu,
            MF_STRING,
            ID_CHOOSE_KEY_CONFIG,
            w!("Choose key config..."),
        )
        .map_err(operation_error)?;
        AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()).map_err(operation_error)?;
        AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quit")).map_err(operation_error)?;

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, cursor.x, cursor.y, None, hwnd, None);
        DestroyMenu(menu).map_err(operation_error)?;
    }

    Ok(())
}

pub(super) fn handle_command(hwnd: HWND, command: TrayMenuCommand) {
    match command {
        TrayMenuCommand::Pause => toggle_pause(hwnd),
        TrayMenuCommand::StartAtLogin => toggle_start_at_login(hwnd),
        TrayMenuCommand::ChooseKeyConfig => choose_key_config(hwnd),
        TrayMenuCommand::Quit => window::request_shutdown(hwnd, ShutdownReason::TrayQuit),
    }
}

fn checked_flag(checked: bool) -> MENU_ITEM_FLAGS {
    if checked { MF_CHECKED } else { MF_UNCHECKED }
}

fn toggle_pause(hwnd: HWND) {
    let Some(state) = STATE.get() else {
        return;
    };
    let Ok(mut state) = state.lock() else {
        return;
    };
    if state.paused && !state.has_config {
        icon::show_notification(
            hwnd,
            "No valid key config is loaded. Choose a key config file first.",
        );
        return;
    }

    let paused = !state.paused;
    if state
        .runtime_tx
        .send(RuntimeCommand::Pause(paused))
        .is_err()
    {
        icon::show_notification(hwnd, "The KeyZen remapping runtime is unavailable.");
        return;
    }
    state.paused = paused;
    icon::update_tooltip(hwnd, paused);
}

fn toggle_start_at_login(hwnd: HWND) {
    let Some(state) = STATE.get() else {
        return;
    };
    let Ok(mut state) = state.lock() else {
        return;
    };
    let enabled = !state.settings.start_at_login;
    let paths = state.paths.clone();
    let tray_exe = state.tray_exe.clone();
    if let Err(error) = update_start_at_login(
        &TaskSchedulerStartup,
        &paths,
        &mut state.settings,
        enabled,
        &tray_exe,
    ) {
        icon::show_notification(hwnd, &format!("Could not update Start at login.\n{error}"));
    }
}

fn choose_key_config(hwnd: HWND) {
    let path = match pick_key_config(hwnd) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(error) => {
            icon::show_notification(hwnd, &format!("Could not open the file picker.\n{error}"));
            return;
        }
    };
    let config = match app_settings::validate_key_config(&path) {
        Ok(config) => config,
        Err(error) => {
            icon::show_notification(
                hwnd,
                &format!("The selected key config is invalid.\n{error}"),
            );
            return;
        }
    };
    let mapped_keys = config.mapping_count();

    let Some(state) = STATE.get() else {
        return;
    };
    let Ok(mut state) = state.lock() else {
        return;
    };
    let resuming = !state.has_config && state.paused;
    if state
        .runtime_tx
        .send(RuntimeCommand::ReplaceConfig(config))
        .is_err()
    {
        icon::show_notification(hwnd, "The KeyZen remapping runtime is unavailable.");
        return;
    }
    state.has_config = true;
    state.config_path = path.clone();
    state.mapped_keys = mapped_keys;
    state.settings.key_config_path = path;
    if resuming && state.runtime_tx.send(RuntimeCommand::Pause(false)).is_ok() {
        state.paused = false;
        icon::update_tooltip(hwnd, false);
        tracing::info!(
            target: "keyzen_win::lifecycle",
            "remapping resumed after loading a valid key config"
        );
    }
    if let Err(error) = app_settings::save_at(&state.paths, &state.settings) {
        icon::show_notification(
            hwnd,
            &format!("The key config was loaded but the app setting was not saved.\n{error}"),
        );
    } else if mapped_keys == 0 {
        icon::show_notification(hwnd, "The selected key config has no mappings.");
    }
}

fn pick_key_config(hwnd: HWND) -> windows::core::Result<Option<PathBuf>> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        let filters = [
            COMDLG_FILTERSPEC {
                pszName: w!("YAML files"),
                pszSpec: w!("*.yaml;*.yml"),
            },
            COMDLG_FILTERSPEC {
                pszName: w!("All files"),
                pszSpec: w!("*.*"),
            },
        ];
        dialog.SetFileTypes(&filters)?;
        dialog.SetTitle(w!("Choose KeyZen key config"))?;
        if let Err(error) = dialog.Show(Some(hwnd)) {
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(error);
        }

        let item = dialog.GetResult()?;
        let raw_path = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = raw_path.to_string();
        CoTaskMemFree(Some(raw_path.0.cast::<c_void>()));
        Ok(Some(PathBuf::from(path?)))
    }
}
