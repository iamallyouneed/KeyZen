mod debug;
mod emitter;
mod hook;
mod keys;
mod runtime;

pub use emitter::SendInputEmitter;

use crate::{KeyzenWinError, RuntimeCommand};
use debug::outcome_has_debug_info;
use keyzen_core::Config;
use runtime::Runtime;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock, mpsc};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_QUIT, WM_USER,
};

const TIMER_MAX_SLEEP_MS: u64 = 50;

static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

pub fn run_controlled(
    config: Option<Config>,
    paused: bool,
    commands: mpsc::Receiver<RuntimeCommand>,
) -> Result<(), KeyzenWinError> {
    let runtime = Arc::new(Runtime::new(config, paused));
    let _ = RUNTIME.set(runtime.clone());

    let thread_id = unsafe { GetCurrentThreadId() };
    // Create this thread's message queue up front. PostThreadMessageW fails on a thread
    // without a queue, and the command thread can request Stop before the loop starts.
    let mut scratch = MSG::default();
    unsafe {
        PeekMessageW(&mut scratch, null_mut(), WM_USER, WM_USER, PM_NOREMOVE);
    }
    let command_runtime = runtime.clone();
    let command_thread = thread::spawn(move || {
        while command_runtime.running.load(Ordering::SeqCst) {
            match commands.recv_timeout(Duration::from_millis(TIMER_MAX_SLEEP_MS)) {
                Ok(RuntimeCommand::Pause(paused)) => command_runtime.set_paused(paused),
                Ok(RuntimeCommand::ReplaceConfig(config)) => {
                    command_runtime.replace_config(config);
                }
                Ok(RuntimeCommand::Stop) => {
                    command_runtime.set_paused(true);
                    if unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) } == 0 {
                        warn!(
                            target: "keyzen_win::lifecycle",
                            error = %last_error("PostThreadMessageW"),
                            "failed to stop remapping runtime message loop"
                        );
                    }
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    warn!(
                        target: "keyzen_win::lifecycle",
                        "remapping runtime command channel disconnected"
                    );
                    command_runtime.set_paused(true);
                    if unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) } == 0 {
                        warn!(
                            target: "keyzen_win::lifecycle",
                            error = %last_error("PostThreadMessageW"),
                            "failed to stop remapping runtime message loop"
                        );
                    }
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    });

    let timer_runtime = runtime.clone();
    let timer_thread = thread::spawn(move || poll_timer(timer_runtime));

    let keyboard_hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook::keyboard_proc), null_mut(), 0) };
    if keyboard_hook.is_null() {
        runtime.running.store(false, Ordering::SeqCst);
        let _ = command_thread.join();
        let _ = timer_thread.join();
        let error = last_error("SetWindowsHookExW");
        error!(
            target: "keyzen_win::lifecycle",
            %error,
            "failed to install low-level keyboard hook"
        );
        return Err(error);
    }
    info!(
        target: "keyzen_win::lifecycle",
        "low-level keyboard hook installed"
    );

    let result = message_loop();
    if let Err(error) = &result {
        error!(
            target: "keyzen_win::lifecycle",
            %error,
            "remapping runtime message loop failed"
        );
    }

    if unsafe { UnhookWindowsHookEx(keyboard_hook) } == 0 {
        warn!(
            target: "keyzen_win::lifecycle",
            error = %last_error("UnhookWindowsHookEx"),
            "failed to remove low-level keyboard hook"
        );
    }
    runtime.running.store(false, Ordering::SeqCst);
    let _ = command_thread.join();
    let _ = timer_thread.join();

    result
}

fn message_loop() -> Result<(), KeyzenWinError> {
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == -1 {
            return Err(last_error("GetMessageW"));
        }
        if result == 0 || message.message == WM_QUIT {
            return Ok(());
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn poll_timer(runtime: Arc<Runtime>) {
    while runtime.running.load(Ordering::SeqCst) {
        if runtime.paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(TIMER_MAX_SLEEP_MS));
            continue;
        }

        let sleep_ms = runtime
            .with_engine(|engine| {
                let now = runtime.now_ms();
                engine
                    .next_deadline_ms()
                    .map(|deadline| deadline.saturating_sub(now).clamp(1, TIMER_MAX_SLEEP_MS))
                    .unwrap_or(TIMER_MAX_SLEEP_MS)
            })
            .unwrap_or(TIMER_MAX_SLEEP_MS);

        thread::sleep(Duration::from_millis(sleep_ms));

        if runtime.paused.load(Ordering::SeqCst) {
            continue;
        }

        let now = runtime.now_ms();
        let (outcome, layers) = runtime
            .with_engine(|engine| {
                let outcome = engine.poll(now);
                let layers = if runtime.debug_events && outcome_has_debug_info(&outcome) {
                    engine.active_layers().to_vec()
                } else {
                    Vec::new()
                };
                (outcome, layers)
            })
            .unwrap_or_default();
        runtime.log_timer_event(now, &outcome, &layers);
        runtime.emit_outcome(outcome);
    }
}

pub(super) fn last_error(api: &'static str) -> KeyzenWinError {
    KeyzenWinError::WindowsApi {
        api,
        code: unsafe { GetLastError() },
    }
}
