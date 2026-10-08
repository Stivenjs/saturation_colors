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
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            receiver: None,
            current: Hotkey::default(),
        }
    }

    pub fn register(&mut self, hotkey: Hotkey) -> Result<(), String> {
        #[cfg(windows)]
        {
            let (sender, receiver) = mpsc::channel();
            windows::register(hotkey, sender)?;
            self.receiver = Some(receiver);
        }
        #[cfg(not(windows))]
        {
            let _ = hotkey;
            self.receiver = None;
        }
        self.current = hotkey;
        Ok(())
    }

    pub fn current(&self) -> Hotkey {
        self.current
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
    use std::{sync::mpsc::Sender, thread};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    pub fn register(hotkey: Hotkey, sender: Sender<HotkeyEvent>) -> Result<(), String> {
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
                    }
                }
                UnregisterHotKey(std::ptr::null_mut(), 1);
            })
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
