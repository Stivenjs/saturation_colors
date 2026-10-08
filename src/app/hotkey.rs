use eframe::egui::{self, Key};

use crate::platform::Hotkey;

use super::ColorApp;

impl ColorApp {
    pub(super) fn capture_hotkey(&mut self, context: &egui::Context) {
        let event = context.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        let Some((key, modifiers)) = event else {
            return;
        };
        let Some(virtual_key) = virtual_key(key) else {
            self.status = "Esa tecla no se puede usar como atajo".to_string();
            return;
        };
        let hotkey = Hotkey {
            modifiers: modifiers_to_win32(modifiers),
            key: virtual_key,
        };
        match self.hotkey_manager.register(hotkey) {
            Ok(()) => {
                self.capturing_hotkey = false;
                self.status = format!("Atajo actualizado: {}", hotkey_label(hotkey));
            }
            Err(error) => self.status = error,
        }
    }
}

fn modifiers_to_win32(modifiers: egui::Modifiers) -> u32 {
    u32::from(modifiers.alt)
        | (u32::from(modifiers.ctrl) * 0x0002)
        | (u32::from(modifiers.shift) * 0x0004)
        | (u32::from(modifiers.mac_cmd) * 0x0008)
}

fn virtual_key(key: Key) -> Option<u32> {
    match key {
        Key::A => Some(b'A' as u32),
        Key::B => Some(b'B' as u32),
        Key::C => Some(b'C' as u32),
        Key::D => Some(b'D' as u32),
        Key::E => Some(b'E' as u32),
        Key::F => Some(b'F' as u32),
        Key::G => Some(b'G' as u32),
        Key::H => Some(b'H' as u32),
        Key::I => Some(b'I' as u32),
        Key::J => Some(b'J' as u32),
        Key::K => Some(b'K' as u32),
        Key::L => Some(b'L' as u32),
        Key::M => Some(b'M' as u32),
        Key::N => Some(b'N' as u32),
        Key::O => Some(b'O' as u32),
        Key::P => Some(b'P' as u32),
        Key::Q => Some(b'Q' as u32),
        Key::R => Some(b'R' as u32),
        Key::S => Some(b'S' as u32),
        Key::T => Some(b'T' as u32),
        Key::U => Some(b'U' as u32),
        Key::V => Some(b'V' as u32),
        Key::W => Some(b'W' as u32),
        Key::X => Some(b'X' as u32),
        Key::Y => Some(b'Y' as u32),
        Key::Z => Some(b'Z' as u32),
        Key::F1 => Some(0x70),
        Key::F2 => Some(0x71),
        Key::F3 => Some(0x72),
        Key::F4 => Some(0x73),
        Key::F5 => Some(0x74),
        Key::F6 => Some(0x75),
        Key::F7 => Some(0x76),
        Key::F8 => Some(0x77),
        Key::F9 => Some(0x78),
        Key::F10 => Some(0x79),
        Key::F11 => Some(0x7A),
        Key::F12 => Some(0x7B),
        _ => None,
    }
}

pub(super) fn hotkey_label(hotkey: Hotkey) -> String {
    let mut label = String::new();
    if hotkey.modifiers & 0x0002 != 0 {
        label.push_str("Ctrl+");
    }
    if hotkey.modifiers & 0x0001 != 0 {
        label.push_str("Alt+");
    }
    if hotkey.modifiers & 0x0004 != 0 {
        label.push_str("Shift+");
    }
    if hotkey.modifiers & 0x0008 != 0 {
        label.push_str("Win+");
    }
    if (0x70..=0x7B).contains(&hotkey.key) {
        label.push_str(&format!("F{}", hotkey.key - 0x6F));
    } else if let Some(character) = char::from_u32(hotkey.key) {
        label.push(character);
    } else {
        label.push('?');
    }
    label
}
