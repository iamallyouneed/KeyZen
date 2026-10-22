use super::RUNTIME;
use super::keys::key_from_keyboard;
use keyzen_core::{Event, EventKind};
use std::ptr::null_mut;
use std::sync::atomic::Ordering;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const LLKHF_INJECTED: u32 = 0x0000_0010;

pub(super) unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32 {
        let keyboard = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if keyboard.flags & LLKHF_INJECTED == 0
            && let Some(event) = keyboard_event(wparam, keyboard)
            && let Some(runtime) = RUNTIME.get()
        {
            if runtime.paused.load(Ordering::SeqCst) {
                return unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) };
            }
            let event_for_log = runtime.debug_events.then(|| event.clone());
            let (outcome, layers) = runtime
                .with_engine(|engine| {
                    let outcome = engine.handle_event(event);
                    let layers = if runtime.debug_events {
                        engine.active_layers().to_vec()
                    } else {
                        Vec::new()
                    };
                    (outcome, layers)
                })
                .unwrap_or_default();
            let suppress = outcome.suppress;
            if let Some(event) = event_for_log.as_ref() {
                runtime.log_key_event(event, &outcome, &layers);
            }
            runtime.emit_outcome(outcome);
            if suppress {
                return 1;
            }
        }
    }

    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn keyboard_event(wparam: WPARAM, keyboard: &KBDLLHOOKSTRUCT) -> Option<Event> {
    let kind = match wparam as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => EventKind::KeyDown,
        WM_KEYUP | WM_SYSKEYUP => EventKind::KeyUp,
        _ => return None,
    };

    Some(Event {
        key: key_from_keyboard(keyboard)?,
        kind,
        time_ms: RUNTIME.get().map(|runtime| runtime.now_ms()).unwrap_or(0),
    })
}
