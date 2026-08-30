//! Global hotkey registration.
//!
//! Works natively on X11 and Windows. On Wayland-native sessions no client can
//! grab global keys by design; bind the compositor/DE to `invoka toggle`
//! instead (see README).

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::bridge;
use crate::config::Config;

/// Build the configured `HotKey` from `config.toml`.
pub fn configured_hotkey() -> HotKey {
    let hotkey = Config::load().hotkey;
    let modifiers = modifiers_from_names(&hotkey.modifiers);
    let modifiers = if modifiers.is_empty() { None } else { Some(modifiers) };
    HotKey::new(modifiers, code_from_name(&hotkey.code))
}

fn modifiers_from_names(names: &[String]) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    for name in names {
        match name.to_lowercase().as_str() {
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "meta" | "logo" | "win" | "cmd" => modifiers |= Modifiers::SUPER,
            _ => {}
        }
    }
    modifiers
}

fn code_from_name(name: &str) -> Code {
    let normalized = name.to_lowercase();
    match normalized.as_str() {
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "tab" => Code::Tab,
        "esc" | "escape" => Code::Escape,
        "backspace" => Code::Backspace,
        "minus" => Code::Minus,
        "equal" => Code::Equal,
        "up" => Code::ArrowUp,
        "down" => Code::ArrowDown,
        "left" => Code::ArrowLeft,
        "right" => Code::ArrowRight,
        canonical if canonical.starts_with("key") && canonical.len() == 4 => {
            key_letter(canonical.chars().last().expect("checked len"))
        }
        canonical if canonical.starts_with("digit") && canonical.len() == 6 => {
            key_digit(canonical.chars().last().expect("checked len"))
        }
        single if single.len() == 1 && single.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) => {
            key_letter(single.chars().next().expect("checked len"))
        }
        digit if digit.len() == 1 && digit.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
            key_digit(digit.chars().next().expect("checked len"))
        }
        normalized if is_fkey(&normalized) => key_fkey(&normalized),
        _ => {
            eprintln!("[invoka] unknown hotkey code '{name}', using space");
            Code::Space
        }
    }
}

fn is_fkey(name: &str) -> bool {
    name.len() >= 2
        && name.starts_with('f')
        && name[1..]
            .chars()
            .all(|c| c.is_ascii_digit())
        && name[1..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n))
}

fn key_fkey(name: &str) -> Code {
    match name {
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        _ => Code::F12,
    }
}

fn key_letter(letter: char) -> Code {
    match letter {
        'a' => Code::KeyA,
        'b' => Code::KeyB,
        'c' => Code::KeyC,
        'd' => Code::KeyD,
        'e' => Code::KeyE,
        'f' => Code::KeyF,
        'g' => Code::KeyG,
        'h' => Code::KeyH,
        'i' => Code::KeyI,
        'j' => Code::KeyJ,
        'k' => Code::KeyK,
        'l' => Code::KeyL,
        'm' => Code::KeyM,
        'n' => Code::KeyN,
        'o' => Code::KeyO,
        'p' => Code::KeyP,
        'q' => Code::KeyQ,
        'r' => Code::KeyR,
        's' => Code::KeyS,
        't' => Code::KeyT,
        'u' => Code::KeyU,
        'v' => Code::KeyV,
        'w' => Code::KeyW,
        'x' => Code::KeyX,
        'y' => Code::KeyY,
        _ => Code::KeyZ,
    }
}

fn key_digit(digit: char) -> Code {
    match digit {
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        _ => Code::Digit9,
    }
}

/// Register the global hotkey and forward presses to the Qt event loop.
///
/// Returns whether registration succeeded.
#[cfg(not(windows))]
pub fn start() -> bool {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("[invoka] global hotkey unavailable: {err}");
            return false;
        }
    };

    let hotkey = configured_hotkey();
    if let Err(err) = manager.register(hotkey) {
        eprintln!("[invoka] failed to register hotkey: {err}");
        return false;
    }

    // Keep the manager alive for the process lifetime; dropping it would
    // unregister the hotkey.
    std::mem::forget(manager);

    std::thread::spawn(move || {
        for event in GlobalHotKeyEvent::receiver() {
            if event.id() == hotkey.id() && event.state == HotKeyState::Pressed {
                bridge::toggle_window();
            }
        }
    });

    true
}

/// Windows variant: the `GlobalHotKeyManager` must be created on (and its
/// hidden window owned by) a thread pumping the Win32 message loop.
#[cfg(windows)]
pub fn start() -> bool {
    let hotkey = configured_hotkey();

    // Owner thread: registers the hotkey and pumps messages forever.
    std::thread::spawn(move || {
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(err) => {
                eprintln!("[invoka] global hotkey unavailable: {err}");
                return;
            }
        };

        if let Err(err) = manager.register(hotkey) {
            eprintln!("[invoka] failed to register hotkey: {err}");
            return;
        }

        // Keep the manager alive for the process lifetime; dropping it would
        // unregister the hotkey.
        std::mem::forget(manager);

        let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = unsafe {
            std::mem::zeroed()
        };
        unsafe {
            // GetMessageW returns -1 on error, 0 on WM_QUIT, > 0 otherwise.
            while windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW(
                &mut msg,
                std::ptr::null_mut(),
                0,
                0,
            ) > 0
            {
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }
    });

    // Listener thread: forwards crate events to the Qt event loop.
    std::thread::spawn(move || {
        for event in GlobalHotKeyEvent::receiver() {
            if event.id() == hotkey.id() && event.state == HotKeyState::Pressed {
                bridge::toggle_window();
            }
        }
    });

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifier_names() {
        let modifiers = modifiers_from_names(&["ctrl".into(), "shift".into()]);
        assert!(modifiers.contains(Modifiers::CONTROL));
        assert!(modifiers.contains(Modifiers::SHIFT));
        assert!(!modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn unknown_modifier_is_ignored() {
        assert_eq!(
            modifiers_from_names(&["bogus".into()]),
            Modifiers::empty()
        );
    }

    #[test]
    fn maps_key_codes() {
        assert_eq!(code_from_name("space"), Code::Space);
        assert_eq!(code_from_name("p"), Code::KeyP);
        assert_eq!(code_from_name("KeyP"), Code::KeyP);
        assert_eq!(code_from_name("7"), Code::Digit7);
        assert_eq!(code_from_name("f5"), Code::F5);
        assert_eq!(code_from_name("escape"), Code::Escape);
    }

    #[test]
    fn unknown_code_falls_back_to_space() {
        assert_eq!(code_from_name("bogus"), Code::Space);
    }

    #[test]
    fn fkey_detection() {
        assert!(is_fkey("f1"));
        assert!(is_fkey("f12"));
        assert!(!is_fkey("f13"));
        assert!(!is_fkey("fx"));
        assert!(!is_fkey("f"));
    }
}
