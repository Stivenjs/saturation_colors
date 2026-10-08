mod filter_controls;
mod hotkey;
mod image_actions;
mod ui;

use eframe::egui::{self, TextureHandle};
use image::RgbaImage;

use crate::{
    application::{PreviewWorker, load_adjustments},
    domain::{ColorAdjustments, ImageData},
    platform::{DisplayFilter, HotkeyManager, create_display_filter},
    tray::{TrayAction, TrayController},
};

pub struct ColorApp {
    source: Option<ImageData>,
    preview: Option<RgbaImage>,
    source_texture: Option<TextureHandle>,
    preview_texture: Option<TextureHandle>,
    adjustments: ColorAdjustments,
    display_filter: Box<dyn DisplayFilter>,
    display_filter_enabled: bool,
    status: String,
    show_original: bool,
    preview_worker: PreviewWorker,
    generation: u64,
    hotkey_manager: HotkeyManager,
    capturing_hotkey: bool,
    selected_target: usize,
    tray: Option<TrayController>,
    allow_exit: bool,
}

impl ColorApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let mut hotkey_manager = HotkeyManager::new(creation_context.egui_ctx.clone());
        let saved_hotkey = HotkeyManager::load_saved();
        if let Err(error) = hotkey_manager.register(saved_hotkey) {
            tracing::warn!(%error, "no se pudo registrar el atajo global");
        }
        let adjustments = load_adjustments();
        Self {
            source: None,
            preview: None,
            source_texture: None,
            preview_texture: None,
            adjustments,
            display_filter: create_display_filter(),
            display_filter_enabled: false,
            status: "Abre una imagen para comenzar".to_string(),
            show_original: false,
            preview_worker: PreviewWorker::new(),
            generation: 0,
            hotkey_manager,
            capturing_hotkey: false,
            selected_target: 0,
            tray: TrayController::new(creation_context.egui_ctx.clone())
                .map_err(|error| tracing::warn!(%error, "no se pudo crear el icono del tray"))
                .ok(),
            allow_exit: false,
        }
    }
}

impl eframe::App for ColorApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_preview(context);
        if self.hotkey_manager.poll_toggle() {
            self.toggle_display_filter();
        }
        if self.capturing_hotkey {
            self.capture_hotkey(context);
            context.request_repaint();
        }
        if self
            .tray
            .as_ref()
            .is_some_and(TrayController::take_exit_request)
        {
            tracing::info!("confirmando cierre definitivo solicitado desde el tray");
            self.allow_exit = true;
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(action) = self.tray.as_ref().and_then(TrayController::poll) {
            match action {
                TrayAction::Show => {
                    context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    context.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                TrayAction::Exit => {
                    self.allow_exit = true;
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if context.input(|input| input.viewport().close_requested()) {
            if self.allow_exit {
                tracing::info!("cierre definitivo permitido; restaurando filtros");
                self.restore_display_filter();
            } else {
                tracing::info!("cierre de ventana interceptado; ocultando aplicación en el tray");
                context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        egui::TopBottomPanel::top("toolbar").show(context, |ui| {
            self.render_toolbar(context, ui);
        });
        egui::SidePanel::right("controls")
            .default_width(330.0)
            .min_width(300.0)
            .max_width(360.0)
            .resizable(false)
            .show(context, |ui| self.render_controls(context, ui));
        egui::CentralPanel::default().show(context, |ui| self.render_canvas(ui));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.restore_display_filter();
    }
}

impl Drop for ColorApp {
    fn drop(&mut self) {
        self.restore_display_filter();
    }
}
