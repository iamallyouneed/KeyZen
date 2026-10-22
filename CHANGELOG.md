# Changelog

## Unreleased

- Consolidate the two executables into a single console-less `keyzen.exe`. The tray runtime is now the only entry point; the launcher, foreground remapper, `validate`/`dump` subcommands, `--config`, and `--debug-events` are removed.
- Merge the `keyzen-cli` crate into `keyzen-win`: one crate now builds the `keyzen` binary, and `cargo run` still starts the tray.
- Split the Windows runtime into focused modules (`platform/{hook,runtime,emitter,keys,debug}`) and the tray into `tray/platform/{window,menu,icon}`.
- Replace the duplicated key-name and virtual-key tables with a single table, covered by round-trip tests.
- Centralize the workspace version in `[workspace.package]`; `package.ps1` reads it from the workspace manifest.
- Set `KEYZEN_DEBUG_EVENTS=1` to write structured key event diagnostics to `keyzen.log` instead of the console.
- Restore the tray icon after Explorer restarts instead of trusting the existing registration state.
- Remove the running marker when a session ends through Quit, logoff, or a hidden-window close, even if the remapping runtime reported an error; the next start no longer reports a false unclean shutdown.
- Show a tray notification when the remapping runtime stops unexpectedly instead of failing silently.
- Resume remapping after choosing a valid config when the tray started paused because no config was loaded.
- Notify through the running tray instead of exiting silently when KeyZen is launched twice.
- Bound the shutdown wait to 5 seconds so Quit cannot hang the process.
- Only report Start at login reconciliation failures when the feature is enabled.
- Keep validating the bundled example configs in `cargo test --workspace`.
- Upgrade note: quit the running `keyzen-tray.exe` before replacing files. The next launch re-registers the `\KeyZen\Autorun for <user>` task against `keyzen.exe`.

## 1.0.0 - 2026-06-08

- Initial Windows release with foreground CLI and console-less tray runtime.
- YAML configuration for stack-based layers, `layer_while_held`, `layer_toggle`, tap-hold, tap dance, one-shot layer, one-shot modifier, and `noop`.
- Native Windows keyboard capture through `WH_KEYBOARD_LL` and output through `SendInput`.
- Tray pause, key config selection, and per-user Start at login through Windows Task Scheduler.

## 1.0.1 - 2026-06-11

- Fix Tray tooltip

## 1.0.2 - 2026-06-14

- Add persistent tray lifecycle and failure diagnostics.
