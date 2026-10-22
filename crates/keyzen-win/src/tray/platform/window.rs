use super::icon::{self, WM_TRAY_ICON};
use super::{ALREADY_RUNNING, STATE, TASKBAR_CREATED, current_paused, menu, operation_error};
use crate::tray::{TrayError, command_from_menu_id};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY,
    WM_ENDSESSION, WM_QUERYENDSESSION, WM_RBUTTONUP, WM_WINDOWPOSCHANGING, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_POPUP,
};
use windows::core::{Error as WindowsError, w};

pub(super) fn create_hidden_window() -> Result<HWND, TrayError> {
    let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    TASKBAR_CREATED
        .set(taskbar_created)
        .map_err(|_| operation_error("taskbar message was already registered"))?;
    let already_running = unsafe { RegisterWindowMessageW(w!("KeyZenShowAlreadyRunning")) };
    if already_running != 0 {
        let _ = ALREADY_RUNNING.set(already_running);
    }

    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| operation_error(format!("GetModuleHandleW failed: {error}")))?;
    let class_name = w!("KeyZenTrayWindow");
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(operation_error(format!(
            "RegisterClassW failed: {}",
            WindowsError::from_win32()
        )));
    }

    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("KeyZen"),
            WS_OVERLAPPEDWINDOW | WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )
        .map_err(|error| operation_error(format!("CreateWindowExW failed: {error}")))
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if TASKBAR_CREATED.get().copied() == Some(message) {
        tracing::info!(
            target: "keyzen_win::lifecycle",
            "TaskbarCreated received; restoring tray icon"
        );
        if icon::try_add_tray_icon(hwnd, current_paused(), true) {
            tracing::info!(target: "keyzen_win::lifecycle", "tray icon restored");
        } else {
            tracing::warn!(
                target: "keyzen_win::lifecycle",
                "failed to restore tray icon after TaskbarCreated"
            );
        }
        return LRESULT(0);
    }

    if ALREADY_RUNNING.get().copied() == Some(message) {
        icon::show_notification(hwnd, "KeyZen is already running.");
        return LRESULT(0);
    }

    match message {
        WM_WINDOWPOSCHANGING => {
            let _ = icon::try_add_tray_icon(hwnd, current_paused(), false);
            LRESULT(0)
        }
        WM_TRAY_ICON
            if lparam.0 as u32 & 0xffff == WM_RBUTTONUP
                || lparam.0 as u32 & 0xffff == WM_CONTEXTMENU =>
        {
            let _ = menu::show_context_menu(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            if let Some(command) = command_from_menu_id(wparam.0 & 0xffff) {
                menu::handle_command(hwnd, command);
            }
            LRESULT(0)
        }
        WM_QUERYENDSESSION => {
            tracing::info!(
                target: "keyzen_win::lifecycle",
                "Windows session end requested"
            );
            LRESULT(1)
        }
        WM_ENDSESSION if wparam.0 != 0 => {
            if let Some(state) = STATE.get()
                && let Ok(mut state) = state.lock()
            {
                state.shutdown_reason = Some("windows_session_ending");
            }
            tracing::info!(
                target: "keyzen_win::lifecycle",
                "Windows session is ending"
            );
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_CLOSE => {
            if let Some(state) = STATE.get()
                && let Ok(mut state) = state.lock()
            {
                state.shutdown_reason = Some("hidden_window_close_message");
            }
            tracing::warn!(
                target: "keyzen_win::lifecycle",
                "tray hidden window received WM_CLOSE"
            );
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = STATE.get()
                && let Ok(mut state) = state.lock()
                && state.shutdown_reason.is_none()
            {
                state.shutdown_reason = Some("hidden_window_destroyed");
                tracing::warn!(
                    target: "keyzen_win::lifecycle",
                    "tray hidden window was destroyed without a tray Quit request"
                );
            }
            // Remove the icon while the window handle is still valid, then quit.
            icon::delete_tray_icon(hwnd);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
