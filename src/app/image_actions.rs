use std::sync::Arc;

use eframe::egui::{self, ColorImage, TextureOptions};
use image::RgbaImage;

use crate::{
    application::{PreviewRequest, load_image, load_profile, save_profile},
    domain::ColorProfile,
};

use super::ColorApp;

impl ColorApp {
    pub(super) fn open_image(&mut self, context: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
        else {
            return;
        };

        match load_image(&path) {
            Ok(image) => {
                self.source = Some(image);
                self.update_source_texture(context);
                self.reprocess(context);
                self.status = format!("Imagen cargada: {}", self.source_name());
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    pub(super) fn source_name(&self) -> &str {
        self.source
            .as_ref()
            .map_or("imagen", |image| image.source_name.as_str())
    }

    pub(super) fn export_image(&mut self) {
        let Some(preview) = &self.preview else {
            self.status = "No hay una imagen procesada para exportar".to_string();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("saturation-colors.png")
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        self.status = match preview.save(&path) {
            Ok(()) => format!("Imagen exportada: {}", path.display()),
            Err(error) => format!("No se pudo exportar: {error}"),
        };
    }

    pub(super) fn save_current_profile(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("mi-perfil.json")
            .add_filter("Perfil JSON", &["json"])
            .save_file()
        else {
            return;
        };
        let profile = ColorProfile::new("Mi perfil", self.adjustments);
        self.status = match save_profile(&path, &profile) {
            Ok(()) => format!("Perfil guardado: {}", path.display()),
            Err(error) => format!("No se pudo guardar el perfil: {error}"),
        };
    }

    pub(super) fn load_current_profile(&mut self, context: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Perfil JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        match load_profile(&path) {
            Ok(profile) => {
                self.adjustments = profile.adjustments;
                self.persist_adjustments();
                self.reprocess(context);
                self.status = format!("Perfil cargado: {}", path.display());
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    pub(super) fn reprocess(&mut self, context: &egui::Context) {
        let Some(source) = &self.source else {
            return;
        };
        self.generation = self.generation.wrapping_add(1);
        self.preview_worker.request(PreviewRequest {
            generation: self.generation,
            image: Arc::clone(&source.image),
            adjustments: self.adjustments,
        });
        context.request_repaint();
    }

    pub(super) fn poll_preview(&mut self, context: &egui::Context) {
        let Some(result) = self.preview_worker.try_recv_latest() else {
            return;
        };
        if result.generation != self.generation {
            context.request_repaint();
            return;
        }
        match result.image {
            Ok(preview) => {
                self.preview = Some(preview);
                self.update_preview_texture(context);
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn update_preview_texture(&mut self, context: &egui::Context) {
        if let Some(preview) = &self.preview {
            self.preview_texture = Some(context.load_texture(
                "preview-image",
                to_color_image(preview),
                TextureOptions::LINEAR,
            ));
        }
    }

    fn update_source_texture(&mut self, context: &egui::Context) {
        if let Some(source) = &self.source {
            self.source_texture = Some(context.load_texture(
                "source-image",
                to_color_image(&source.image),
                TextureOptions::LINEAR,
            ));
        }
    }
}

fn to_color_image(image: &RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}
