use std::sync::mpsc::{self, Receiver};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub enum TrayAction {
    Show,
    Exit,
}

pub struct TrayController {
    _tray_icon: TrayIcon,
    show_id: tray_icon::menu::MenuId,
    exit_id: tray_icon::menu::MenuId,
    receiver: Receiver<tray_icon::menu::MenuEvent>,
    exit_requested: Arc<AtomicBool>,
}

impl TrayController {
    pub fn new(context: eframe::egui::Context) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();

        let menu = Menu::new();
        let show = MenuItem::new("Mostrar Saturation Colors", true, None);
        let exit = MenuItem::new("Salir", true, None);
        let show_id = show.id().clone();
        let exit_id = exit.id().clone();
        let show_id_callback = show_id.clone();
        let exit_id_callback = exit_id.clone();
        let exit_requested = Arc::new(AtomicBool::new(false));
        let exit_requested_callback = Arc::clone(&exit_requested);
        MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
            tracing::info!(event_id = ?event.id, "evento recibido desde el tray");
            let _ = sender.send(event.clone());
            context.request_repaint();
            if event.id == show_id_callback {
                tracing::info!("mostrando la ventana desde el tray");
                Self::show_window();
            } else if event.id == exit_id_callback {
                tracing::info!("solicitud de salida recibida desde el tray");
                exit_requested_callback.store(true, Ordering::Release);
            } else {
                tracing::debug!("evento de menú del tray ignorado");
            }
        }));
        menu.append(&show).map_err(|error| error.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|error| error.to_string())?;
        menu.append(&exit).map_err(|error| error.to_string())?;

        let icon = Icon::from_rgba(vec![0x32, 0x9d, 0xff, 0xff], 1, 1)
            .map_err(|error| error.to_string())?;
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Saturation Colors")
            .with_icon(icon)
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            _tray_icon: tray_icon,
            show_id,
            exit_id,
            receiver,
            exit_requested,
        })
    }

    pub fn poll(&self) -> Option<TrayAction> {
        let event = self.receiver.try_iter().last()?;
        if event.id == self.show_id {
            Some(TrayAction::Show)
        } else if event.id == self.exit_id {
            Some(TrayAction::Exit)
        } else {
            None
        }
    }

    pub fn take_exit_request(&self) -> bool {
        self.exit_requested.swap(false, Ordering::AcqRel)
    }

    #[cfg(windows)]
    fn window_handle() -> windows_sys::Win32::Foundation::HWND {
        let title: Vec<u16> = "Saturation Colors\0".encode_utf16().collect();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
                std::ptr::null(),
                title.as_ptr(),
            )
        }
    }

    #[cfg(windows)]
    fn show_window() {
        let hwnd = Self::window_handle();
        if hwnd.is_null() {
            tracing::warn!("no se encontró la ventana para mostrarla desde el tray");
            return;
        }
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
            };
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
}
