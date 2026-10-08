use eframe::egui;

use crate::application::save_adjustments;

use super::ColorApp;

impl ColorApp {
    pub(super) fn reset(&mut self, context: &egui::Context) {
        self.adjustments = crate::domain::ColorAdjustments::default();
        self.persist_adjustments();
        self.reprocess(context);
        self.status = if self.display_filter_enabled {
            match self.display_filter.apply(self.adjustments) {
                Ok(()) => "Ajustes locales y filtro global restablecidos".to_string(),
                Err(error) => format!(
                    "Ajustes locales restablecidos, pero no se pudo actualizar el filtro global: {error}"
                ),
            }
        } else {
            "Ajustes restablecidos".to_string()
        };
    }

    pub(super) fn persist_adjustments(&self) {
        if let Err(error) = save_adjustments(self.adjustments) {
            tracing::warn!(%error, "no se pudieron guardar los ajustes de color");
        }
    }

    pub(super) fn toggle_display_filter(&mut self) {
        if self.display_filter_enabled {
            self.status = match self.display_filter.restore() {
                Ok(()) => {
                    self.display_filter_enabled = false;
                    "Filtro global desactivado".to_string()
                }
                Err(error) => {
                    self.display_filter_enabled = true;
                    error.to_string()
                }
            };
        } else {
            self.status = match self.display_filter.apply(self.adjustments) {
                Ok(()) => {
                    self.display_filter_enabled = true;
                    "Filtro global activado".to_string()
                }
                Err(error) => error.to_string(),
            };
        }
    }

    pub(super) fn restore_display_filter(&mut self) {
        match self.display_filter.restore() {
            Ok(()) => {
                self.display_filter_enabled = false;
            }
            Err(error) => {
                tracing::error!(%error, "no se pudo restaurar el filtro de pantalla al salir");
            }
        }
    }
}
