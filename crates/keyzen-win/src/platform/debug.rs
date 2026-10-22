use keyzen_core::{Event, EventKind, KeyCode, Outcome, OutputCommand};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct KeyEventDebugData<'a> {
    pub(super) time_ms: u64,
    pub(super) input: &'a KeyCode,
    pub(super) kind: EventKind,
    pub(super) suppress: bool,
    pub(super) outputs: &'a [OutputCommand],
    pub(super) layers: &'a [String],
}

pub(super) fn key_event_debug_data<'a>(
    event: &'a Event,
    outcome: &'a Outcome,
    layers: &'a [String],
) -> KeyEventDebugData<'a> {
    KeyEventDebugData {
        time_ms: event.time_ms,
        input: &event.key,
        kind: event.kind,
        suppress: outcome.suppress,
        outputs: &outcome.commands,
        layers,
    }
}

pub(super) fn outcome_has_debug_info(outcome: &Outcome) -> bool {
    !outcome.commands.is_empty() || !outcome.diagnostics.is_empty()
}

pub(super) fn format_output_commands(commands: &[OutputCommand]) -> String {
    let commands = commands
        .iter()
        .map(|command| match command {
            OutputCommand::KeyDown(key) => format!("KeyDown({key})"),
            OutputCommand::KeyUp(key) => format!("KeyUp({key})"),
            OutputCommand::Tap(key) => format!("Tap({key})"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{commands}]")
}

pub(super) fn format_layers(layers: &[String]) -> String {
    format!("[{}]", layers.join(", "))
}

#[cfg(test)]
mod tests {
    use super::{
        format_layers, format_output_commands, key_event_debug_data, outcome_has_debug_info,
    };
    use keyzen_core::{Diagnostic, Event, EventKind, Outcome, OutputCommand};

    #[test]
    fn key_event_debug_data_contains_input_output_time_and_state() {
        let event = Event {
            key: "A".parse().unwrap(),
            kind: EventKind::KeyDown,
            time_ms: 42,
        };
        let outcome = Outcome {
            suppress: true,
            commands: vec![OutputCommand::Tap("Left".parse().unwrap())],
            diagnostics: Vec::new(),
        };
        let layers = vec!["base".to_string(), "nav".to_string()];

        let data = key_event_debug_data(&event, &outcome, &layers);

        assert_eq!(data.time_ms, 42);
        assert_eq!(data.input.as_str(), "A");
        assert_eq!(data.kind, EventKind::KeyDown);
        assert!(data.suppress);
        assert_eq!(data.outputs, outcome.commands);
        assert_eq!(data.layers, layers);
    }

    #[test]
    fn timer_debug_events_only_log_meaningful_outcomes() {
        assert!(!outcome_has_debug_info(&Outcome::default()));
        assert!(outcome_has_debug_info(&Outcome {
            commands: vec![OutputCommand::Tap("A".parse().unwrap())],
            ..Outcome::default()
        }));
        assert!(outcome_has_debug_info(&Outcome {
            diagnostics: vec![Diagnostic::Warning("warning".to_string())],
            ..Outcome::default()
        }));
    }

    #[test]
    fn formats_debug_outputs_and_layers_for_humans() {
        let commands = vec![
            OutputCommand::KeyDown("LeftCtrl".parse().unwrap()),
            OutputCommand::Tap("Left".parse().unwrap()),
            OutputCommand::KeyUp("LeftCtrl".parse().unwrap()),
        ];
        let layers = vec!["base".to_string(), "nav".to_string()];

        assert_eq!(
            format_output_commands(&commands),
            "[KeyDown(LeftCtrl), Tap(Left), KeyUp(LeftCtrl)]"
        );
        assert_eq!(format_layers(&layers), "[base, nav]");
    }
}
