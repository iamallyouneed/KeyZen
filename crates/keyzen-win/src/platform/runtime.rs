use super::debug::{
    format_layers, format_output_commands, key_event_debug_data, outcome_has_debug_info,
};
use super::emitter::SendInputEmitter;
use crate::{DEBUG_EVENTS_TARGET, Emitter};
use keyzen_core::{Config, Diagnostic, Engine, Event, Outcome};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tracing::{debug, warn};

#[derive(Debug)]
pub(super) struct Runtime {
    engine: Mutex<Option<Engine>>,
    emitter: SendInputEmitter,
    started_at: Instant,
    pub(super) running: AtomicBool,
    pub(super) paused: AtomicBool,
    pub(super) debug_events: bool,
}

impl Runtime {
    pub(super) fn new(config: Option<Config>, paused: bool) -> Self {
        Self {
            engine: Mutex::new(config.map(Engine::new)),
            emitter: SendInputEmitter,
            started_at: Instant::now(),
            running: AtomicBool::new(true),
            paused: AtomicBool::new(paused),
            debug_events: crate::debug_events_enabled(),
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    pub(super) fn has_engine(&self) -> bool {
        self.engine
            .lock()
            .map(|engine| engine.is_some())
            .unwrap_or(false)
    }

    pub(super) fn with_engine<T>(&self, f: impl FnOnce(&mut Engine) -> T) -> Option<T> {
        match self.engine.lock() {
            Ok(mut engine) => engine.as_mut().map(f),
            Err(error) => {
                warn!("engine lock poisoned: {error}");
                None
            }
        }
    }

    pub(super) fn emit_outcome(&self, outcome: Outcome) {
        for diagnostic in outcome.diagnostics {
            match diagnostic {
                Diagnostic::Warning(message) => warn!("{message}"),
            }
        }
        if outcome.commands.is_empty() {
            return;
        }
        if let Err(error) = self.emitter.emit(&outcome.commands) {
            warn!("failed to emit input: {error}");
        }
    }

    pub(super) fn log_key_event(&self, event: &Event, outcome: &Outcome, layers: &[String]) {
        if !self.debug_events {
            return;
        }

        let data = key_event_debug_data(event, outcome, layers);
        debug!(
            target: DEBUG_EVENTS_TARGET,
            time_ms = data.time_ms,
            input = %data.input,
            kind = ?data.kind,
            suppress = data.suppress,
            outputs = %format_output_commands(data.outputs),
            layers = %format_layers(data.layers),
            "key_event"
        );
    }

    pub(super) fn log_timer_event(&self, time_ms: u64, outcome: &Outcome, layers: &[String]) {
        if !self.debug_events || !outcome_has_debug_info(outcome) {
            return;
        }

        debug!(
            target: DEBUG_EVENTS_TARGET,
            time_ms,
            outputs = %format_output_commands(&outcome.commands),
            diagnostics = ?outcome.diagnostics,
            layers = %format_layers(layers),
            "timer_event"
        );
    }

    fn log_reset_event(
        &self,
        reason: &'static str,
        time_ms: u64,
        outcome: &Outcome,
        layers: &[String],
    ) {
        if !self.debug_events || !outcome_has_debug_info(outcome) {
            return;
        }

        debug!(
            target: DEBUG_EVENTS_TARGET,
            time_ms,
            reason,
            outputs = %format_output_commands(&outcome.commands),
            diagnostics = ?outcome.diagnostics,
            layers = %format_layers(layers),
            "reset_event"
        );
    }

    pub(super) fn set_paused(&self, paused: bool) {
        if paused {
            self.paused.store(true, Ordering::SeqCst);
            if let Some((outcome, layers)) = self.with_engine(|engine| {
                let outcome = engine.reset();
                let layers = if self.debug_events && outcome_has_debug_info(&outcome) {
                    engine.active_layers().to_vec()
                } else {
                    Vec::new()
                };
                (outcome, layers)
            }) {
                self.log_reset_event("pause", self.now_ms(), &outcome, &layers);
                self.emit_outcome(outcome);
            }
        } else if self.has_engine() {
            self.paused.store(false, Ordering::SeqCst);
        }
    }

    pub(super) fn replace_config(&self, config: Config) {
        let mut reset = None;
        if let Ok(mut engine) = self.engine.lock() {
            if let Some(current) = engine.as_mut() {
                let outcome = current.reset();
                let layers = if self.debug_events && outcome_has_debug_info(&outcome) {
                    current.active_layers().to_vec()
                } else {
                    Vec::new()
                };
                reset = Some((outcome, layers));
            }
            *engine = Some(Engine::new(config));
        }
        if let Some((outcome, layers)) = reset {
            self.log_reset_event("replace_config", self.now_ms(), &outcome, &layers);
            self.emit_outcome(outcome);
        }
    }
}
