use super::keys::{shifted_key, vk_for_key};
use super::last_error;
use crate::{Emitter, KeyzenWinError};
use keyzen_core::OutputCommand;
use std::mem::size_of;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct SendInputEmitter;

impl Emitter for SendInputEmitter {
    fn emit(&self, commands: &[OutputCommand]) -> Result<(), KeyzenWinError> {
        for command in expand_commands(commands)? {
            send_keyboard_input(command)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyboardInput {
    vk: u16,
    key_up: bool,
}

fn expand_commands(commands: &[OutputCommand]) -> Result<Vec<KeyboardInput>, KeyzenWinError> {
    let mut inputs = Vec::new();
    for command in commands {
        match command {
            OutputCommand::KeyDown(key) => inputs.push(KeyboardInput {
                vk: vk_for_key(key)?,
                key_up: false,
            }),
            OutputCommand::KeyUp(key) => inputs.push(KeyboardInput {
                vk: vk_for_key(key)?,
                key_up: true,
            }),
            OutputCommand::Tap(key) => {
                if let Some((modifier, shifted_key)) = shifted_key(key) {
                    inputs.push(KeyboardInput {
                        vk: modifier,
                        key_up: false,
                    });
                    inputs.push(KeyboardInput {
                        vk: shifted_key,
                        key_up: false,
                    });
                    inputs.push(KeyboardInput {
                        vk: shifted_key,
                        key_up: true,
                    });
                    inputs.push(KeyboardInput {
                        vk: modifier,
                        key_up: true,
                    });
                } else {
                    let vk = vk_for_key(key)?;
                    inputs.push(KeyboardInput { vk, key_up: false });
                    inputs.push(KeyboardInput { vk, key_up: true });
                }
            }
        }
    }
    Ok(inputs)
}

fn send_keyboard_input(input: KeyboardInput) -> Result<(), KeyzenWinError> {
    let mut flags = 0;
    if input.key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    let raw = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: input.vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(1, &raw, size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(last_error("SendInput"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{KeyboardInput, expand_commands};
    use keyzen_core::OutputCommand;

    #[test]
    fn expands_shifted_tap() {
        let commands = vec![OutputCommand::Tap("DoubleQuote".parse().unwrap())];
        let expanded = expand_commands(&commands).unwrap();

        assert_eq!(
            expanded,
            vec![
                KeyboardInput {
                    vk: 0x10,
                    key_up: false
                },
                KeyboardInput {
                    vk: 0xDE,
                    key_up: false
                },
                KeyboardInput {
                    vk: 0xDE,
                    key_up: true
                },
                KeyboardInput {
                    vk: 0x10,
                    key_up: true
                },
            ]
        );
    }
}
