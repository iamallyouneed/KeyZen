use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{IDI_APPLICATION, LoadIconW, WM_APP};

pub(super) const WM_TRAY_ICON: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;

static TRAY_ICON_CREATED: AtomicBool = AtomicBool::new(false);

/// Adds the icon once. `force_restore` re-registers it even when the flag says it
/// already exists, which is required after Explorer restarts and drops every icon.
pub(super) fn try_add_tray_icon(hwnd: HWND, paused: bool, force_restore: bool) -> bool {
    if !force_restore && TRAY_ICON_CREATED.load(Ordering::SeqCst) {
        return true;
    }

    let Ok(icon) = (unsafe { LoadIconW(None, IDI_APPLICATION) }) else {
        tracing::warn!(
            target: "keyzen_win::lifecycle",
            "failed to load tray icon"
        );
        return false;
    };
    let mut data = base_notify_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    data.hIcon = icon;
    fill_wide(&mut data.szTip, tooltip(paused));

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        TRAY_ICON_CREATED.store(false, Ordering::SeqCst);
        tracing::warn!(
            target: "keyzen_win::lifecycle",
            error = %std::io::Error::last_os_error(),
            "failed to register tray icon"
        );
        return false;
    }
    TRAY_ICON_CREATED.store(true, Ordering::SeqCst);
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) }.as_bool() {
        tracing::warn!(
            target: "keyzen_win::lifecycle",
            error = %std::io::Error::last_os_error(),
            "failed to set tray icon notification version"
        );
    }
    true
}

pub(super) fn delete_tray_icon(hwnd: HWND) {
    if !TRAY_ICON_CREATED.swap(false, Ordering::SeqCst) {
        return;
    }
    let data = base_notify_data(hwnd);
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
}

pub(super) fn update_tooltip(hwnd: HWND, paused: bool) {
    let mut data = base_notify_data(hwnd);
    data.uFlags = NIF_TIP | NIF_SHOWTIP;
    fill_wide(&mut data.szTip, tooltip(paused));
    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
}

pub(super) fn show_notification(hwnd: HWND, message: &str) {
    let mut data = base_notify_data(hwnd);
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = NIIF_INFO;
    fill_wide(&mut data.szInfoTitle, "KeyZen");
    fill_wide(&mut data.szInfo, message);
    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
}

fn base_notify_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    }
}

fn tooltip(paused: bool) -> &'static str {
    if paused { "KeyZen (Paused)" } else { "KeyZen" }
}

fn fill_wide<const N: usize>(target: &mut [u16; N], value: &str) {
    target.fill(0);
    for (slot, code_unit) in target
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(value.encode_utf16())
    {
        *slot = code_unit;
    }
}
