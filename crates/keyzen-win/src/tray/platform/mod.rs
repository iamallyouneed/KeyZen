mod icon;
mod menu;
mod window;

use crate::RuntimeCommand;
use crate::app_settings::{self, AppPaths, AppSettings};
use crate::diagnostics::TrayDiagnostics;
use crate::single_instance::{SingleInstance, SingleInstanceError};
use crate::startup::{StartupRegistration, TaskSchedulerStartup};
use crate::tray::{TrayError, is_clean_shutdown_reason};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, FindWindowW, GetMessageW, MSG, PostMessageW, RegisterWindowMessageW,
    TranslateMessage,
};
use windows::core::{Error as WindowsError, w};

static STATE: OnceLock<Mutex<TrayState>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static ALREADY_RUNNING: OnceLock<u32> = OnceLock::new();

struct TrayState {
    paths: AppPaths,
    settings: AppSettings,
    runtime_tx: mpsc::Sender<RuntimeCommand>,
    paused: bool,
    has_config: bool,
    tray_exe: PathBuf,
    shutdown_reason: Option<&'static str>,
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn run_tray_app() -> Result<(), TrayError> {
    let mut diagnostics = AppPaths::discover()
        .ok()
        .and_then(|paths| TrayDiagnostics::initialize(&paths).ok());
    let _instance = match SingleInstance::acquire() {
        Ok(instance) => instance,
        Err(SingleInstanceError::AlreadyRunning) => {
            tracing::info!(
                target: "keyzen_win::lifecycle",
                "tray launch ignored because another KeyZen instance is running"
            );
            notify_existing_instance();
            return Ok(());
        }
        Err(error) => {
            tracing::error!(
                target: "keyzen_win::lifecycle",
                %error,
                "failed to acquire the KeyZen single-instance mutex"
            );
            return Err(TrayError::Operation(error.to_string()));
        }
    };

    if let Some(diagnostics) = diagnostics.as_mut()
        && let Err(error) = diagnostics.begin_session()
    {
        tracing::error!(
            target: "keyzen_win::lifecycle",
            %error,
            "failed to create tray session marker"
        );
    }
    tracing::info!(
        target: "keyzen_win::lifecycle",
        pid = std::process::id(),
        "tray process started"
    );

    let result = run_tray_session();
    let clean_shutdown = session_ended_cleanly();
    match &result {
        Ok(()) => tracing::info!(
            target: "keyzen_win::lifecycle",
            clean_shutdown,
            "tray process exited normally"
        ),
        Err(error) => tracing::error!(
            target: "keyzen_win::lifecycle",
            %error,
            clean_shutdown,
            "tray process exited with an error"
        ),
    }
    if let Some(diagnostics) = diagnostics {
        diagnostics.complete(clean_shutdown);
    }
    result
}

fn run_tray_session() -> Result<(), TrayError> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|error| operation_error(format!("CoInitializeEx failed: {error}")))?;
    }
    let _com = ComGuard;

    let (paths, settings) =
        app_settings::load_or_bootstrap().map_err(|error| operation_error(error.to_string()))?;
    let (config, paused, startup_notice) =
        match app_settings::load_key_config(&paths.settings, &settings) {
            Ok(config) => (Some(config), false, None),
            Err(error) => (
                None,
                true,
                Some(format!(
                    "Key config is invalid or missing. Choose a key config file.\n{error}"
                )),
            ),
        };

    let tray_exe = std::env::current_exe()
        .map_err(|error| operation_error(format!("current_exe failed: {error}")))?;
    let start_at_login = settings.start_at_login;
    let scheduler_notice = match TaskSchedulerStartup.set_enabled(start_at_login, &tray_exe) {
        Ok(()) => None,
        Err(error) => {
            tracing::warn!(
                target: "keyzen_win::lifecycle",
                %error,
                start_at_login,
                "failed to reconcile Start at login"
            );
            start_at_login.then(|| format!("Could not reconcile Start at login.\n{error}"))
        }
    };

    let (runtime_tx, runtime_rx) = mpsc::channel();
    STATE
        .set(Mutex::new(TrayState {
            paths,
            settings,
            runtime_tx,
            paused,
            has_config: !paused,
            tray_exe,
            shutdown_reason: None,
        }))
        .map_err(|_| operation_error("tray state was already initialized"))?;

    let hwnd = window::create_hidden_window()?;
    if icon::try_add_tray_icon(hwnd, paused, false) {
        tracing::info!(target: "keyzen_win::lifecycle", "tray icon registered");
    }
    // HWND wraps a raw pointer, so pass its value across the thread boundary.
    let hwnd_bits = hwnd.0 as usize;
    let runtime = thread::spawn(move || {
        tracing::info!(target: "keyzen_win::lifecycle", "remapping runtime thread started");
        let result = crate::run_controlled(config, paused, runtime_rx);
        match &result {
            Ok(()) => tracing::info!(
                target: "keyzen_win::lifecycle",
                "remapping runtime thread stopped"
            ),
            Err(error) => {
                tracing::error!(
                    target: "keyzen_win::lifecycle",
                    %error,
                    "remapping runtime thread failed"
                );
                if !shutdown_requested() {
                    icon::show_notification(
                        HWND(hwnd_bits as *mut c_void),
                        &format!("KeyZen keyboard remapping stopped.\n{error}"),
                    );
                }
            }
        }
        result
    });

    if let Some(message) = startup_notice.or(scheduler_notice) {
        icon::show_notification(hwnd, &message);
    }

    let mut message = MSG::default();
    let mut message_error = None;
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 == -1 {
            message_error = Some(operation_error(WindowsError::from_win32()));
            break;
        }
        if result.0 == 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    let shutdown_reason = STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.shutdown_reason)
        .unwrap_or("message_loop_ended_without_destroy");
    tracing::info!(
        target: "keyzen_win::lifecycle",
        shutdown_reason,
        "tray message loop stopped"
    );
    icon::delete_tray_icon(hwnd);
    if let Some(state) = STATE.get()
        && let Ok(state) = state.lock()
    {
        let _ = state.runtime_tx.send(RuntimeCommand::Stop);
    }
    if let Some(error) = message_error {
        let _ = join_runtime(runtime);
        return Err(error);
    }
    join_runtime(runtime)
}

fn session_ended_cleanly() -> bool {
    STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.shutdown_reason)
        .is_some_and(is_clean_shutdown_reason)
}

const RUNTIME_STOP_TIMEOUT: Duration = Duration::from_secs(5);

fn join_runtime(
    runtime: thread::JoinHandle<Result<(), crate::KeyzenWinError>>,
) -> Result<(), TrayError> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(runtime.join());
    });
    match rx.recv_timeout(RUNTIME_STOP_TIMEOUT) {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) => Err(operation_error(error)),
        Ok(Err(_)) => Err(operation_error("KeyZen runtime thread panicked")),
        Err(_) => {
            tracing::error!(
                target: "keyzen_win::lifecycle",
                timeout_ms = RUNTIME_STOP_TIMEOUT.as_millis() as u64,
                "remapping runtime did not stop in time"
            );
            Err(operation_error("KeyZen runtime did not stop in time"))
        }
    }
}

fn notify_existing_instance() {
    let message = unsafe { RegisterWindowMessageW(w!("KeyZenShowAlreadyRunning")) };
    if message == 0 {
        return;
    }
    let Ok(hwnd) = (unsafe { FindWindowW(w!("KeyZenTrayWindow"), w!("KeyZen")) }) else {
        return;
    };
    unsafe {
        let _ = PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0));
    }
}

fn shutdown_requested() -> bool {
    STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.shutdown_reason)
        .is_some()
}

pub(super) fn current_paused() -> bool {
    STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.paused)
        .unwrap_or(true)
}

pub(super) fn operation_error(error: impl ToString) -> TrayError {
    TrayError::Operation(error.to_string())
}
