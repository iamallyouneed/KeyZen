use crate::KeyzenWinError;
use keyzen_core::KeyCode;
use windows_sys::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT;

const VK_SHIFT: u16 = 0x10;
const LLKHF_EXTENDED: u32 = 0x0000_0001;

/// Canonical key names and their virtual-key codes. `key_from_vk` uses the first
/// entry that matches a code, so keep one canonical name per code.
const KEY_CODES: &[(&str, u16)] = &[
    ("Backspace", 0x08),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Shift", 0x10),
    ("Ctrl", 0x11),
    ("Alt", 0x12),
    ("Pause", 0x13),
    ("CapsLock", 0x14),
    ("Escape", 0x1B),
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("0", 0x30),
    ("1", 0x31),
    ("2", 0x32),
    ("3", 0x33),
    ("4", 0x34),
    ("5", 0x35),
    ("6", 0x36),
    ("7", 0x37),
    ("8", 0x38),
    ("9", 0x39),
    ("A", 0x41),
    ("B", 0x42),
    ("C", 0x43),
    ("D", 0x44),
    ("E", 0x45),
    ("F", 0x46),
    ("G", 0x47),
    ("H", 0x48),
    ("I", 0x49),
    ("J", 0x4A),
    ("K", 0x4B),
    ("L", 0x4C),
    ("M", 0x4D),
    ("N", 0x4E),
    ("O", 0x4F),
    ("P", 0x50),
    ("Q", 0x51),
    ("R", 0x52),
    ("S", 0x53),
    ("T", 0x54),
    ("U", 0x55),
    ("V", 0x56),
    ("W", 0x57),
    ("X", 0x58),
    ("Y", 0x59),
    ("Z", 0x5A),
    ("LeftWin", 0x5B),
    ("RightWin", 0x5C),
    ("Menu", 0x5D),
    ("NumLock", 0x90),
    ("ScrollLock", 0x91),
    ("LeftShift", 0xA0),
    ("RightShift", 0xA1),
    ("LeftCtrl", 0xA2),
    ("RightCtrl", 0xA3),
    ("LeftAlt", 0xA4),
    ("RightAlt", 0xA5),
    ("Semicolon", 0xBA),
    ("Equal", 0xBB),
    ("Comma", 0xBC),
    ("Minus", 0xBD),
    ("Period", 0xBE),
    ("Slash", 0xBF),
    ("Grave", 0xC0),
    ("LeftBracket", 0xDB),
    ("Backslash", 0xDC),
    ("RightBracket", 0xDD),
    ("Quote", 0xDE),
];

/// Names that are typed with Shift plus a base virtual-key.
const SHIFTED_KEYS: &[(&str, u16)] = &[
    ("DoubleQuote", 0xDE),
    ("Colon", 0xBA),
    ("LessThan", 0xBC),
    ("GreaterThan", 0xBE),
    ("Question", 0xBF),
    ("Pipe", 0xDC),
    ("Underscore", 0xBD),
    ("Plus", 0xBB),
    ("Tilde", 0xC0),
    ("LeftBrace", 0xDB),
    ("RightBrace", 0xDD),
    ("Exclamation", 0x31),
    ("At", 0x32),
    ("Hash", 0x33),
    ("Dollar", 0x34),
    ("Percent", 0x35),
    ("Caret", 0x36),
    ("Ampersand", 0x37),
    ("Asterisk", 0x38),
    ("LeftParen", 0x39),
    ("RightParen", 0x30),
];

pub(super) fn key_from_keyboard(keyboard: &KBDLLHOOKSTRUCT) -> Option<KeyCode> {
    match keyboard.vkCode as u16 {
        0x10 => {
            if keyboard.scanCode == 0x36 {
                KeyCode::new("RightShift").ok()
            } else {
                KeyCode::new("LeftShift").ok()
            }
        }
        0x11 => {
            if keyboard.flags & LLKHF_EXTENDED != 0 {
                KeyCode::new("RightCtrl").ok()
            } else {
                KeyCode::new("LeftCtrl").ok()
            }
        }
        0x12 => {
            if keyboard.flags & LLKHF_EXTENDED != 0 {
                KeyCode::new("RightAlt").ok()
            } else {
                KeyCode::new("LeftAlt").ok()
            }
        }
        vk => key_from_vk(vk),
    }
}

fn key_from_vk(vk: u16) -> Option<KeyCode> {
    if let Some((name, _)) = KEY_CODES.iter().find(|(_, code)| *code == vk) {
        return KeyCode::new(*name).ok();
    }
    if (0x70..=0x87).contains(&vk) {
        return KeyCode::new(format!("F{}", vk - 0x6F)).ok();
    }
    None
}

pub(super) fn vk_for_key(key: &KeyCode) -> Result<u16, KeyzenWinError> {
    let name = key.as_str();
    if let Some((_, vk)) = KEY_CODES.iter().find(|(candidate, _)| *candidate == name) {
        return Ok(*vk);
    }
    if name == "Win" {
        return Ok(0x5B);
    }
    if let Some(number) = name
        .strip_prefix('F')
        .and_then(|rest| rest.parse::<u16>().ok())
        && (1..=24).contains(&number)
    {
        return Ok(0x6F + number);
    }
    Err(KeyzenWinError::UnsupportedKey(name.to_string()))
}

pub(super) fn shifted_key(key: &KeyCode) -> Option<(u16, u16)> {
    SHIFTED_KEYS
        .iter()
        .find(|(name, _)| *name == key.as_str())
        .map(|(_, vk)| (VK_SHIFT, *vk))
}

#[cfg(test)]
mod tests {
    use super::{
        KEY_CODES, SHIFTED_KEYS, VK_SHIFT, key_from_keyboard, key_from_vk, shifted_key, vk_for_key,
    };
    use keyzen_core::KeyCode;
    use std::collections::BTreeSet;
    use windows_sys::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT;

    #[test]
    fn key_codes_round_trip_through_virtual_keys() {
        for (name, vk) in KEY_CODES {
            let key = KeyCode::new(*name).unwrap();
            assert_eq!(vk_for_key(&key).unwrap(), *vk, "virtual key for {name}");
            assert_eq!(
                key_from_vk(*vk).unwrap().as_str(),
                *name,
                "key name for {vk:#04X}"
            );
        }
    }

    #[test]
    fn key_codes_use_one_canonical_name_per_virtual_key() {
        let mut seen = BTreeSet::new();
        for (name, vk) in KEY_CODES {
            assert!(
                seen.insert(*vk),
                "duplicate virtual key {vk:#04X} at {name}"
            );
        }
    }

    #[test]
    fn function_keys_round_trip() {
        for number in 1..=24u16 {
            let key = KeyCode::new(format!("F{number}")).unwrap();
            assert_eq!(vk_for_key(&key).unwrap(), 0x6F + number);
            assert_eq!(key_from_vk(0x6F + number).unwrap().as_str(), key.as_str());
        }
    }

    #[test]
    fn win_alias_maps_to_left_win() {
        let alias = KeyCode::new("Win").unwrap();

        assert_eq!(vk_for_key(&alias).unwrap(), 0x5B);
        assert_eq!(key_from_vk(0x5B).unwrap().as_str(), "LeftWin");
    }

    #[test]
    fn rejects_unknown_keys() {
        let unknown = KeyCode::new("NotAKey").unwrap();

        assert!(vk_for_key(&unknown).is_err());
        assert!(key_from_vk(0xFE).is_none());
    }

    #[test]
    fn normalizes_modifier_sides_from_low_level_keyboard_data() {
        let right_shift = KBDLLHOOKSTRUCT {
            vkCode: 0x10,
            scanCode: 0x36,
            flags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
        let right_ctrl = KBDLLHOOKSTRUCT {
            vkCode: 0x11,
            scanCode: 0,
            flags: 0x01,
            time: 0,
            dwExtraInfo: 0,
        };

        assert_eq!(
            key_from_keyboard(&right_shift).unwrap().as_str(),
            "RightShift"
        );
        assert_eq!(
            key_from_keyboard(&right_ctrl).unwrap().as_str(),
            "RightCtrl"
        );
    }

    #[test]
    fn shifted_keys_are_typed_with_shift() {
        for (name, vk) in SHIFTED_KEYS {
            let key = KeyCode::new(*name).unwrap();
            assert_eq!(shifted_key(&key), Some((VK_SHIFT, *vk)), "{name}");
        }
    }
}
