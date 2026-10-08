use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    pub modifiers: u32,
    pub key: u32,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            modifiers: 0x0002 | 0x0004,
            key: 0x7B,
        }
    }
}

pub enum HotkeyEvent {
    Toggle,
}

pub struct HotkeyManager {
    receiver: Option<Receiver<HotkeyEvent>>,
    current: Hotkey,
    context: egui::Context,
}

impl HotkeyManager {
    pub fn new(context: egui::Context) -> Self {
        Self {
            receiver: None,
            current: Hotkey::default(),
            context,
        }
    }

    pub fn load_saved() -> Hotkey {
        let Some(path) = Self::settings_path() else {
            return Hotkey::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(hotkey) => hotkey,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "configuración de atajo inválida");
                    Hotkey::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Hotkey::default(),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "no se pudo leer el atajo guardado");
                Hotkey::default()
            }
        }
    }

    pub fn register(&mut self, hotkey: Hotkey) -> Result<(), String> {
        #[cfg(windows)]
        {
            let (sender, receiver) = mpsc::channel();
            windows::register(hotkey, sender, self.context.clone())?;
            self.receiver = Some(receiver);
        }
        #[cfg(not(windows))]
        {
            let _ = hotkey;
            self.receiver = None;
        }
        self.current = hotkey;
        if let Err(error) = Self::save_hotkey(hotkey) {
            tracing::warn!(%error, "no se pudo guardar el atajo global");
        }
        Ok(())
    }

    pub fn current(&self) -> Hotkey {
        self.current
    }

    fn settings_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("com", "Saturation Colors", "Saturation Colors")
            .map(|directories| directories.config_dir().join("hotkey.json"))
    }

    fn save_hotkey(hotkey: Hotkey) -> Result<(), String> {
        let path = Self::settings_path()
            .ok_or_else(|| "no se pudo determinar la carpeta de configuración".to_string())?;
        let parent = path
            .parent()
            .ok_or_else(|| "la ruta de configuración no tiene carpeta padre".to_string())?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let contents = serde_json::to_string_pretty(&hotkey).map_err(|error| error.to_string())?;
        std::fs::write(path, contents).map_err(|error| error.to_string())
    }

    pub fn poll_toggle(&self) -> bool {
        let Some(receiver) = &self.receiver else {
            return false;
        };
        match receiver.try_recv() {
            Ok(HotkeyEvent::Toggle) => true,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => false,
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::{Hotkey, HotkeyEvent};
    use eframe::egui;
    use std::{sync::mpsc::Sender, thread};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    pub fn register(
        hotkey: Hotkey,
        sender: Sender<HotkeyEvent>,
        context: egui::Context,
    ) -> Result<(), String> {
        thread::Builder::new()
            .name("global-hotkey".to_string())
            .spawn(move || unsafe {
                if RegisterHotKey(
                    std::ptr::null_mut(),
                    1,
                    hotkey.modifiers | MOD_NOREPEAT,
                    hotkey.key,
                ) == 0
                {
                    return;
                }
                let mut message = MSG::default();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    if message.message == WM_HOTKEY {
                        let _ = sender.send(HotkeyEvent::Toggle);
                        context.request_repaint();
                    }
                }
                UnregisterHotKey(std::ptr::null_mut(), 1);
            })
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
