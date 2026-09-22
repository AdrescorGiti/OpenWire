use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rdev::{EventType, Key};

use crate::audio::mixer::MixerControl;
use crate::soundboard::SoundManager;

#[derive(Clone)]
pub struct HotkeyService {
    mapping: Arc<RwLock<HashMap<Key, (usize, usize)>>>,
    panic_key: Arc<RwLock<Key>>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            mapping: Arc::new(RwLock::new(default_mapping())),
            panic_key: Arc::new(RwLock::new(Key::Escape)),
        }
    }

    pub fn rebind(&self, bank: usize, slot: usize, key: Key) {
        let mut map = self.mapping.write().expect("hotkey lock");
        map.retain(|_, v| *v != (bank, slot));
        map.insert(key, (bank, slot));
    }

    pub fn clear_binding(&self, bank: usize, slot: usize) {
        self.mapping
            .write()
            .expect("hotkey lock")
            .retain(|_, value| *value != (bank, slot));
    }

    pub fn rebind_label(&self, bank: usize, slot: usize, label: &str) -> Result<(), String> {
        let key =
            parse_key_label(label).ok_or_else(|| format!("unsupported hotkey label: {label:?}"))?;
        self.rebind(bank, slot, key);
        Ok(())
    }

    pub fn sync_from_board(&self, board: &crate::soundboard::bank::Soundboard) {
        let mut map = default_mapping();
        for bank in &board.banks {
            for pad in &bank.pads {
                if let Some(label) = &pad.hotkey {
                    if let Some(key) = parse_key_label(label) {
                        map.insert(key, (pad.bank, pad.id));
                    }
                }
            }
        }
        *self.mapping.write().expect("hotkey lock") = map;
    }

    #[allow(dead_code)]
    pub fn mapping(&self) -> HashMap<Key, (usize, usize)> {
        self.mapping.read().expect("hotkey lock").clone()
    }

    pub fn spawn_listener(
        &self,
        sound: Arc<SoundManager>,
        control: MixerControl,
    ) -> std::thread::JoinHandle<()> {
        let mapping = self.mapping.clone();
        let panic_key = self.panic_key.clone();
        std::thread::Builder::new()
            .name("openwire-hotkeys".into())
            .spawn(move || {
                let callback = move |event: rdev::Event| {
                    let EventType::KeyPress(key) = event.event_type else {
                        return;
                    };
                    if key == *panic_key.read().expect("panic key lock") {
                        control.stop_all();
                        return;
                    }
                    let binding = {
                        let map = mapping.read().expect("hotkey lock");
                        map.get(&key).copied()
                    };
                    if let Some((bank, slot)) = binding {
                        if let Err(e) = sound.play(bank, slot) {
                            eprintln!("[openwire][hotkeys] pad {bank}/{slot}: {e:#}");
                        }
                    }
                };
                if let Err(err) = rdev::listen(callback) {
                    eprintln!("[openwire][hotkeys] listener unavailable: {err:?}");
                }
            })
            .expect("failed to spawn hotkey thread")
    }
}

impl Default for HotkeyService {
    fn default() -> Self {
        Self::new()
    }
}

fn default_mapping() -> HashMap<Key, (usize, usize)> {
    const KEYS: [Key; 15] = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
        Key::Num0,
        Key::KeyQ,
        Key::KeyW,
        Key::KeyE,
        Key::KeyR,
        Key::KeyT,
    ];
    KEYS.iter()
        .enumerate()
        .map(|(slot, key)| (*key, (0usize, slot)))
        .collect()
}

fn parse_key_label(label: &str) -> Option<Key> {
    let up = label.trim().to_uppercase();
    let key = match up.as_str() {
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "A" => Key::KeyA,
        "B" => Key::KeyB,
        "C" => Key::KeyC,
        "D" => Key::KeyD,
        "E" => Key::KeyE,
        "F" => Key::KeyF,
        "G" => Key::KeyG,
        "H" => Key::KeyH,
        "I" => Key::KeyI,
        "J" => Key::KeyJ,
        "K" => Key::KeyK,
        "L" => Key::KeyL,
        "M" => Key::KeyM,
        "N" => Key::KeyN,
        "O" => Key::KeyO,
        "P" => Key::KeyP,
        "Q" => Key::KeyQ,
        "R" => Key::KeyR,
        "S" => Key::KeyS,
        "T" => Key::KeyT,
        "U" => Key::KeyU,
        "V" => Key::KeyV,
        "W" => Key::KeyW,
        "X" => Key::KeyX,
        "Y" => Key::KeyY,
        "Z" => Key::KeyZ,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "KP0" | "NUMPAD0" => Key::Kp0,
        "KP1" | "NUMPAD1" => Key::Kp1,
        "KP2" | "NUMPAD2" => Key::Kp2,
        "KP3" | "NUMPAD3" => Key::Kp3,
        "KP4" | "NUMPAD4" => Key::Kp4,
        "KP5" | "NUMPAD5" => Key::Kp5,
        "KP6" | "NUMPAD6" => Key::Kp6,
        "KP7" | "NUMPAD7" => Key::Kp7,
        "KP8" | "NUMPAD8" => Key::Kp8,
        "KP9" | "NUMPAD9" => Key::Kp9,
        "SPACE" => Key::Space,
        "TAB" => Key::Tab,
        "ENTER" | "RETURN" => Key::Return,
        "BACKSPACE" => Key::Backspace,
        "DELETE" | "DEL" => Key::Delete,
        "INSERT" | "INS" => Key::Insert,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEUP" | "PAGE UP" | "PGUP" => Key::PageUp,
        "PAGEDOWN" | "PAGE DOWN" | "PGDN" => Key::PageDown,
        "UP" | "ARROWUP" => Key::UpArrow,
        "DOWN" | "ARROWDOWN" => Key::DownArrow,
        "LEFT" | "ARROWLEFT" => Key::LeftArrow,
        "RIGHT" | "ARROWRIGHT" => Key::RightArrow,
        "CAPSLOCK" => Key::CapsLock,
        "PRINTSCREEN" | "PRTSCR" => Key::PrintScreen,
        "SCROLLLOCK" => Key::ScrollLock,
        "PAUSE" => Key::Pause,
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_parse() {
        assert!(matches!(parse_key_label("1"), Some(Key::Num1)));
        assert!(matches!(parse_key_label("q"), Some(Key::KeyQ)));
        assert!(matches!(parse_key_label("F5"), Some(Key::F5)));
        assert!(matches!(parse_key_label("kp7"), Some(Key::Kp7)));
        assert!(parse_key_label("meta-left").is_none());
    }

    #[test]
    fn rebind_moves_binding() {
        let svc = HotkeyService::new();
        svc.rebind_label(2, 4, "G").unwrap();
        let map = svc.mapping();
        assert_eq!(map.get(&Key::KeyG), Some(&(2, 4)));

        assert!(!map.values().filter(|v| **v == (2, 4)).count() > 1);
    }
}
