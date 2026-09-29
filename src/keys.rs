//! Key events in the W3C UI Events vocabulary.
//!
//! A `keyDown`/`pressesBegan` on a view becomes a [`KeyEvent`]: the
//! layout-aware [`Key`] for what the key means, the layout-independent
//! [`Code`] for where it sits, and the modifier chord. No platform keycode
//! leaves the kit — the mapping tables below translate macOS virtual
//! keycodes and `UIKit` HID usages into the same vocabulary.
//!
//! # Safety
//!
//! No unsafe code; the `AppKit`/`UIKit` event accessors used are safe reads.

use keyboard_types::{Code, Key, Modifiers, NamedKey};

/// One key press, in platform-independent terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// What the key means under the current layout and modifiers.
    pub key: Key,
    /// Where the key sits on the keyboard, independent of layout.
    pub code: Code,
    /// The modifiers held with the key.
    pub modifiers: Modifiers,
    /// Whether this press is an auto-repeat of a held key.
    pub repeat: bool,
}

/// Parses a W3C `KeyboardEvent.code` table entry; an entry the vocabulary
/// lacks is `Unidentified`, matching the port's fallback.
fn code_of(name: &str) -> Code {
    name.parse().unwrap_or(Code::Unidentified)
}

/// Parses a W3C `KeyboardEvent.key` name; a name the vocabulary lacks is
/// `Unidentified`.
fn key_of(name: &str) -> Key {
    name.parse().unwrap_or(Key::Named(NamedKey::Unidentified))
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_app_kit::{NSEvent, NSEventModifierFlags};

    use super::{Code, Key, KeyEvent, Modifiers, NamedKey, code_of, key_of};

    /// macOS virtual keycodes (`kVK_*`) to W3C `KeyboardEvent.code` names.
    #[allow(clippy::too_many_lines)]
    const fn code_for_virtual_key(key_code: u16) -> Option<&'static str> {
        Some(match key_code {
            0x00 => "KeyA",
            0x01 => "KeyS",
            0x02 => "KeyD",
            0x03 => "KeyF",
            0x04 => "KeyH",
            0x05 => "KeyG",
            0x06 => "KeyZ",
            0x07 => "KeyX",
            0x08 => "KeyC",
            0x09 => "KeyV",
            0x0A => "IntlBackslash",
            0x0B => "KeyB",
            0x0C => "KeyQ",
            0x0D => "KeyW",
            0x0E => "KeyE",
            0x0F => "KeyR",
            0x10 => "KeyY",
            0x11 => "KeyT",
            0x12 => "Digit1",
            0x13 => "Digit2",
            0x14 => "Digit3",
            0x15 => "Digit4",
            0x16 => "Digit6",
            0x17 => "Digit5",
            0x18 => "Equal",
            0x19 => "Digit9",
            0x1A => "Digit7",
            0x1B => "Minus",
            0x1C => "Digit8",
            0x1D => "Digit0",
            0x1E => "BracketRight",
            0x1F => "KeyO",
            0x20 => "KeyU",
            0x21 => "BracketLeft",
            0x22 => "KeyI",
            0x23 => "KeyP",
            0x24 => "Enter",
            0x25 => "KeyL",
            0x26 => "KeyJ",
            0x27 => "Quote",
            0x28 => "KeyK",
            0x29 => "Semicolon",
            0x2A => "Backslash",
            0x2B => "Comma",
            0x2C => "Slash",
            0x2D => "KeyN",
            0x2E => "KeyM",
            0x2F => "Period",
            0x30 => "Tab",
            0x31 => "Space",
            0x32 => "Backquote",
            0x33 => "Backspace",
            0x35 => "Escape",
            0x36 => "OSRight",
            0x37 => "OSLeft",
            0x38 => "ShiftLeft",
            0x39 => "CapsLock",
            0x3A => "AltLeft",
            0x3B => "ControlLeft",
            0x3C => "ShiftRight",
            0x3D => "AltRight",
            0x3E => "ControlRight",
            0x3F => "Fn",
            0x40 => "F17",
            0x41 => "NumpadDecimal",
            0x43 => "NumpadMultiply",
            0x45 => "NumpadAdd",
            0x47 => "NumLock",
            0x48 => "VolumeUp",
            0x49 => "VolumeDown",
            0x4A => "VolumeMute",
            0x4B => "NumpadDivide",
            0x4C => "NumpadEnter",
            0x4E => "NumpadSubtract",
            0x4F => "F18",
            0x50 => "F19",
            0x51 => "NumpadEqual",
            0x52 => "Numpad0",
            0x53 => "Numpad1",
            0x54 => "Numpad2",
            0x55 => "Numpad3",
            0x56 => "Numpad4",
            0x57 => "Numpad5",
            0x58 => "Numpad6",
            0x59 => "Numpad7",
            0x5A => "F20",
            0x5B => "Numpad8",
            0x5C => "Numpad9",
            0x5D => "IntlYen",
            0x5E => "IntlRo",
            0x5F => "NumpadComma",
            0x60 => "F5",
            0x61 => "F6",
            0x62 => "F7",
            0x63 => "F3",
            0x64 => "F8",
            0x65 => "F9",
            0x66 => "Lang2",
            0x67 => "F11",
            0x68 => "Lang1",
            0x69 => "F13",
            0x6A => "F16",
            0x6B => "F14",
            0x6D => "F10",
            0x6E => "ContextMenu",
            0x6F => "F12",
            0x71 => "F15",
            0x72 => "Help",
            0x73 => "Home",
            0x74 => "PageUp",
            0x75 => "Delete",
            0x76 => "F4",
            0x77 => "End",
            0x78 => "F2",
            0x79 => "PageDown",
            0x7A => "F1",
            0x7B => "ArrowLeft",
            0x7C => "ArrowRight",
            0x7D => "ArrowDown",
            0x7E => "ArrowUp",
            _ => return None,
        })
    }

    /// `AppKit`'s private-use function-key code points to W3C `key` names.
    const fn key_for_function_scalar(value: u32) -> Option<&'static str> {
        Some(match value {
            0xF700 => "ArrowUp",
            0xF701 => "ArrowDown",
            0xF702 => "ArrowLeft",
            0xF703 => "ArrowRight",
            0xF704 => "F1",
            0xF705 => "F2",
            0xF706 => "F3",
            0xF707 => "F4",
            0xF708 => "F5",
            0xF709 => "F6",
            0xF70A => "F7",
            0xF70B => "F8",
            0xF70C => "F9",
            0xF70D => "F10",
            0xF70E => "F11",
            0xF70F => "F12",
            0xF710 => "F13",
            0xF711 => "F14",
            0xF712 => "F15",
            0xF713 => "F16",
            0xF714 => "F17",
            0xF715 => "F18",
            0xF716 => "F19",
            0xF717 => "F20",
            0xF726 => "Insert",
            0xF727 => "Delete",
            0xF728 => "Home",
            0xF729 => "End",
            0xF72A => "PageUp",
            0xF72B => "PageDown",
            0xF72C => "PrintScreen",
            0xF72D => "ScrollLock",
            0xF72E => "Pause",
            0xF72F => "ContextMenu",
            0xF730 => "Help",
            0xF739 => "Clear",
            _ => return None,
        })
    }

    /// Control characters `AppKit` delivers for keys the W3C model names.
    const fn key_for_control_scalar(value: u32) -> Option<&'static str> {
        Some(match value {
            0x0D | 0x03 => "Enter",
            0x09 | 0x19 => "Tab",
            0x1B => "Escape",
            0x7F => "Backspace",
            _ => return None,
        })
    }

    /// The W3C `key` of a modifier key, which is named rather than typed.
    const fn key_for_modifier_code(code: Code) -> Option<&'static str> {
        Some(match code {
            Code::ShiftLeft | Code::ShiftRight => "Shift",
            Code::ControlLeft | Code::ControlRight => "Control",
            Code::AltLeft | Code::AltRight => "Alt",
            Code::MetaLeft | Code::MetaRight => "Meta",
            Code::CapsLock => "CapsLock",
            Code::NumLock => "NumLock",
            Code::Fn => "Fn",
            _ => return None,
        })
    }

    /// The modifier chord of an event, as `Modifiers` bits.
    fn modifiers_of(flags: NSEventModifierFlags) -> Modifiers {
        let mut bits = Modifiers::empty();
        bits.set(
            Modifiers::SHIFT,
            flags.contains(NSEventModifierFlags::Shift),
        );
        bits.set(
            Modifiers::CONTROL,
            flags.contains(NSEventModifierFlags::Control),
        );
        bits.set(Modifiers::ALT, flags.contains(NSEventModifierFlags::Option));
        bits.set(
            Modifiers::META,
            flags.contains(NSEventModifierFlags::Command),
        );
        bits.set(
            Modifiers::CAPS_LOCK,
            flags.contains(NSEventModifierFlags::CapsLock),
        );
        bits.set(
            Modifiers::NUM_LOCK,
            flags.contains(NSEventModifierFlags::NumericPad),
        );
        bits
    }

    /// The W3C `KeyboardEvent.key` — the value the layout and modifiers
    /// produce.
    fn key_for_event(event: &NSEvent, code: Code) -> Key {
        if let Some(name) = key_for_modifier_code(code) {
            return key_of(name);
        }
        let Some(characters) = event.charactersIgnoringModifiers() else {
            return Key::Named(NamedKey::Unidentified);
        };
        let text = characters.to_string();
        let Some(scalar) = text.chars().next() else {
            return Key::Named(NamedKey::Unidentified);
        };
        let value = u32::from(scalar);
        if let Some(name) = key_for_function_scalar(value).or_else(|| key_for_control_scalar(value))
        {
            return key_of(name);
        }
        // A chord such as ⌃A yields the control character, not the letter;
        // the W3C `key` for it is still the letter the physical key types.
        if value < 0x20
            && let Some(unmodified) =
                event.charactersByApplyingModifiers(NSEventModifierFlags::empty())
            && unmodified
                .to_string()
                .chars()
                .next()
                .is_some_and(|c| u32::from(c) >= 0x20)
        {
            return Key::Character(unmodified.to_string());
        }
        Key::Character(text)
    }

    /// The press this `keyDown` event describes.
    #[must_use]
    pub fn key_event(event: &NSEvent) -> KeyEvent {
        let code = code_for_virtual_key(event.keyCode()).map_or(Code::Unidentified, code_of);
        KeyEvent {
            key: key_for_event(event, code),
            code,
            modifiers: modifiers_of(event.modifierFlags()),
            repeat: event.isARepeat(),
        }
    }
}

#[cfg(target_os = "ios")]
mod imp {
    use objc2_ui_kit::{UIKey, UIKeyModifierFlags};

    use super::{Code, Key, KeyEvent, Modifiers, NamedKey, code_of, key_of};

    /// `UIKit` `UIKeyboardHIDUsage` values to W3C `KeyboardEvent.code` names.
    #[allow(clippy::too_many_lines)]
    const fn code_for_hid_usage(usage: i64) -> Option<&'static str> {
        Some(match usage {
            0x04 => "KeyA",
            0x05 => "KeyB",
            0x06 => "KeyC",
            0x07 => "KeyD",
            0x08 => "KeyE",
            0x09 => "KeyF",
            0x0A => "KeyG",
            0x0B => "KeyH",
            0x0C => "KeyI",
            0x0D => "KeyJ",
            0x0E => "KeyK",
            0x0F => "KeyL",
            0x10 => "KeyM",
            0x11 => "KeyN",
            0x12 => "KeyO",
            0x13 => "KeyP",
            0x14 => "KeyQ",
            0x15 => "KeyR",
            0x16 => "KeyS",
            0x17 => "KeyT",
            0x18 => "KeyU",
            0x19 => "KeyV",
            0x1A => "KeyW",
            0x1B => "KeyX",
            0x1C => "KeyY",
            0x1D => "KeyZ",
            0x1E => "Digit1",
            0x1F => "Digit2",
            0x20 => "Digit3",
            0x21 => "Digit4",
            0x22 => "Digit5",
            0x23 => "Digit6",
            0x24 => "Digit7",
            0x25 => "Digit8",
            0x26 => "Digit9",
            0x27 => "Digit0",
            0x28 => "Enter",
            0x29 => "Escape",
            0x2A => "Backspace",
            0x2B => "Tab",
            0x2C => "Space",
            0x2D => "Minus",
            0x2E => "Equal",
            0x2F => "BracketLeft",
            0x30 => "BracketRight",
            0x31 => "Backslash",
            0x33 => "Semicolon",
            0x34 => "Quote",
            0x35 => "Backquote",
            0x36 => "Comma",
            0x37 => "Period",
            0x38 => "Slash",
            0x39 => "CapsLock",
            0x3A => "F1",
            0x3B => "F2",
            0x3C => "F3",
            0x3D => "F4",
            0x3E => "F5",
            0x3F => "F6",
            0x40 => "F7",
            0x41 => "F8",
            0x42 => "F9",
            0x43 => "F10",
            0x44 => "F11",
            0x45 => "F12",
            0x46 => "PrintScreen",
            0x47 => "ScrollLock",
            0x48 => "Pause",
            0x49 => "Insert",
            0x4A => "Home",
            0x4B => "PageUp",
            0x4C => "Delete",
            0x4D => "End",
            0x4E => "PageDown",
            0x4F => "ArrowRight",
            0x50 => "ArrowLeft",
            0x51 => "ArrowDown",
            0x52 => "ArrowUp",
            0x53 => "NumLock",
            0x54 => "NumpadDivide",
            0x55 => "NumpadMultiply",
            0x56 => "NumpadSubtract",
            0x57 => "NumpadAdd",
            0x58 => "NumpadEnter",
            0x59 => "Numpad1",
            0x5A => "Numpad2",
            0x5B => "Numpad3",
            0x5C => "Numpad4",
            0x5D => "Numpad5",
            0x5E => "Numpad6",
            0x5F => "Numpad7",
            0x60 => "Numpad8",
            0x61 => "Numpad9",
            0x62 => "Numpad0",
            0x63 => "NumpadDecimal",
            0x64 => "IntlBackslash",
            0x65 => "ContextMenu",
            0x67 => "NumpadEqual",
            0x68 => "F13",
            0x69 => "F14",
            0x6A => "F15",
            0x6B => "F16",
            0x6C => "F17",
            0x6D => "F18",
            0x6E => "F19",
            0x6F => "F20",
            0x75 => "Help",
            0x85 => "NumpadComma",
            0x87 => "IntlRo",
            0x88 => "Lang1",
            0x89 => "IntlYen",
            0x8A => "Lang2",
            0xE0 => "ControlLeft",
            0xE1 => "ShiftLeft",
            0xE2 => "AltLeft",
            0xE3 => "OSLeft",
            0xE4 => "ControlRight",
            0xE5 => "ShiftRight",
            0xE6 => "AltRight",
            0xE7 => "OSRight",
            _ => return None,
        })
    }

    /// The W3C `key` a HID usage names on its own, before the layout speaks.
    const fn key_for_hid_usage(usage: i64) -> Option<&'static str> {
        Some(match usage {
            0x28 | 0x58 => "Enter",
            0x29 => "Escape",
            0x2A => "Backspace",
            0x2B => "Tab",
            0x39 => "CapsLock",
            0x3A => "F1",
            0x3B => "F2",
            0x3C => "F3",
            0x3D => "F4",
            0x3E => "F5",
            0x3F => "F6",
            0x40 => "F7",
            0x41 => "F8",
            0x42 => "F9",
            0x43 => "F10",
            0x44 => "F11",
            0x45 => "F12",
            0x46 => "PrintScreen",
            0x47 => "ScrollLock",
            0x48 => "Pause",
            0x49 => "Insert",
            0x4A => "Home",
            0x4B => "PageUp",
            0x4C => "Delete",
            0x4D => "End",
            0x4E => "PageDown",
            0x4F => "ArrowRight",
            0x50 => "ArrowLeft",
            0x51 => "ArrowDown",
            0x52 => "ArrowUp",
            0x53 => "NumLock",
            0x65 => "ContextMenu",
            0x68 => "F13",
            0x69 => "F14",
            0x6A => "F15",
            0x6B => "F16",
            0x6C => "F17",
            0x6D => "F18",
            0x6E => "F19",
            0x6F => "F20",
            0x75 => "Help",
            0xE0 | 0xE4 => "Control",
            0xE1 | 0xE5 => "Shift",
            0xE2 | 0xE6 => "Alt",
            0xE3 | 0xE7 => "Meta",
            _ => return None,
        })
    }

    /// The modifier chord of a key press, as `Modifiers` bits.
    fn modifiers_of(flags: UIKeyModifierFlags) -> Modifiers {
        let mut bits = Modifiers::empty();
        bits.set(Modifiers::SHIFT, flags.contains(UIKeyModifierFlags::Shift));
        bits.set(
            Modifiers::CONTROL,
            flags.contains(UIKeyModifierFlags::Control),
        );
        bits.set(
            Modifiers::ALT,
            flags.contains(UIKeyModifierFlags::Alternate),
        );
        bits.set(Modifiers::META, flags.contains(UIKeyModifierFlags::Command));
        bits.set(
            Modifiers::CAPS_LOCK,
            flags.contains(UIKeyModifierFlags::AlphaShift),
        );
        bits.set(
            Modifiers::NUM_LOCK,
            flags.contains(UIKeyModifierFlags::NumericPad),
        );
        bits
    }

    /// The press this `UIPress`'s key describes.
    #[must_use]
    pub fn key_event(key: &UIKey) -> KeyEvent {
        let usage = key.keyCode().0 as i64;
        let code = code_for_hid_usage(usage).map_or(Code::Unidentified, code_of);
        let modifiers = modifiers_of(key.modifierFlags());
        let key = key_for_hid_usage(usage).map_or_else(
            || {
                let characters = key.charactersIgnoringModifiers().to_string();
                if characters.is_empty() {
                    Key::Named(NamedKey::Unidentified)
                } else {
                    key_of(&characters)
                }
            },
            key_of,
        );
        KeyEvent {
            key,
            code,
            modifiers,
            repeat: false,
        }
    }
}

pub use imp::key_event;
