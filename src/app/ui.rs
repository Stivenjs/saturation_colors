use eframe::egui::{self, TextureHandle};

use crate::{domain::ColorAdjustments, platform::detected_gpus};

use super::{ColorApp, hotkey::hotkey_label};

impl ColorApp {
    pub(super) fn render_toolbar(&mut self, context: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.strong("Saturation Colors");
            ui.separator();
            if ui.button("Abrir imagen").clicked() {
                self.open_image(context);
            }
            if ui.button("Exportar").clicked() {
                self.export_image();
            }
            ui.menu_button("Perfiles", |ui| {
                if ui.button("Guardar perfil").clicked() {
                    self.save_current_profile();
                    ui.close();
                }
                if ui.button("Cargar perfil").clicked() {
                    self.load_current_profile(context);
                    ui.close();
                }
            });
            if ui.button("Restablecer").clicked() {
                self.reset(context);
            }
            ui.separator();
            ui.checkbox(&mut self.show_original, "Original");
            ui.separator();
            if ui.button("Configurar atajo").clicked() {
                self.capturing_hotkey = true;
                self.status = "Presiona una tecla con Ctrl, Alt, Shift o Win".to_string();
            }
            ui.small(hotkey_label(self.hotkey_manager.current()));
            let filter_label = if self.display_filter_enabled {
                "Desactivar filtro"
            } else {
                "Activar filtro global"
            };
            ui.add_enabled_ui(
                self.display_filter.is_available() || self.display_filter_enabled,
                |ui| {
                    if ui.button(filter_label).clicked() {
                        self.toggle_display_filter();
                    }
                },
            );
        });
    }

    pub(super) fn render_controls(&mut self, context: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Controles");
        ui.add_space(4.0);
        egui::CollapsingHeader::new("Ajustes de color")
            .default_open(true)
            .show(ui, |ui| {
                let saturation_before = self.adjustments.saturation;
                let mut changed = false;
                changed |= slider(
                    ui,
                    "Brillo",
                    &mut self.adjustments.brightness,
                    ColorAdjustments::BRIGHTNESS_RANGE,
                );
                changed |= slider(
                    ui,
                    "Contraste",
                    &mut self.adjustments.contrast,
                    ColorAdjustments::CONTRAST_RANGE,
                );
                changed |= slider(
                    ui,
                    "Saturación",
                    &mut self.adjustments.saturation,
                    ColorAdjustments::SATURATION_RANGE,
                );
                changed |= slider(
                    ui,
                    "Temperatura",
                    &mut self.adjustments.temperature,
                    ColorAdjustments::TEMPERATURE_RANGE,
                );
                changed |= slider(
                    ui,
                    "Tinte",
                    &mut self.adjustments.tint,
                    ColorAdjustments::TINT_RANGE,
                );
                changed |= slider(
                    ui,
                    "Gamma",
                    &mut self.adjustments.gamma,
                    ColorAdjustments::GAMMA_RANGE,
                );
                if changed {
                    self.reprocess(context);
                    if self.display_filter_enabled {
                        self.status = match self.display_filter.apply(self.adjustments) {
                            Ok(()) => "Filtro global actualizado".to_string(),
                            Err(error) => error.to_string(),
                        };
                        if self
                            .display_filter
                            .backend_name()
                            .starts_with("Windows Gamma")
                            && (self.adjustments.saturation - saturation_before).abs()
                                > f32::EPSILON
                        {
                            self.status =
                                "Vista previa actualizada; la saturación global requiere AMD ADL o NVIDIA NVAPI"
                                    .to_string();
                        }
                    }
                }
            });

        ui.add_space(8.0);
        egui::CollapsingHeader::new("Filtro global")
            .default_open(true)
            .show(ui, |ui| self.render_filter_section(ui));
    }

    fn render_filter_section(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Backend: {}", self.display_filter.backend_name()));
        ui.small(if self.display_filter_enabled {
            "Estado: activo"
        } else {
            "Estado: inactivo"
        });
        if self
            .display_filter
            .backend_name()
            .starts_with("Windows Gamma")
        {
            ui.add_space(4.0);
            ui.colored_label(
                egui::Color32::YELLOW,
                "La gamma de Windows aplica el cambio al monitor seleccionado.",
            );
            ui.small("La saturación requiere un backend nativo AMD ADL o NVIDIA NVAPI.");
        }

        let targets = self.display_filter.targets();
        if !targets.is_empty() {
            ui.label("Destino del filtro");
            let selected_label = targets
                .get(self.selected_target)
                .map_or("Seleccionar destino", |target| target.label.as_str());
            egui::ComboBox::from_label("Destino")
                .selected_text(selected_label)
                .width(215.0)
                .show_ui(ui, |ui| {
                    for (index, target) in targets.iter().enumerate() {
                        if ui
                            .selectable_value(&mut self.selected_target, index, &target.label)
                            .clicked()
                        {
                            self.display_filter.select_target(index);
                            if self.display_filter_enabled {
                                self.status = match self.display_filter.apply(self.adjustments) {
                                    Ok(()) => format!("Aplicado a {}", target.label),
                                    Err(error) => error.to_string(),
                                };
                            }
                        }
                    }
                });
            ui.small("Selecciona el monitor donde se aplicará el filtro.");
        } else {
            ui.label("No hay monitores controlables disponibles.");
        }

        ui.add_space(6.0);
        ui.label("Hardware detectado");
        let gpus = detected_gpus();
        if gpus.is_empty() {
            ui.small("No se detectaron GPUs en Windows.");
        } else {
            for gpu in gpus {
                ui.horizontal(|ui| {
                    ui.label("•");
                    ui.label(gpu);
                    if self
                        .display_filter
                        .backend_name()
                        .starts_with("Windows Gamma")
                    {
                        ui.small("(detectada; control por monitor)");
                    } else {
                        ui.small("(detectada; backend activo)");
                    }
                });
            }
        }

        egui::CollapsingHeader::new("Diagnóstico")
            .default_open(false)
            .show(ui, |ui| {
                ui.small(self.display_filter.diagnostics());
            });
    }

    pub(super) fn render_canvas(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Previsualización");
            ui.add_space(12.0);
            ui.small(&self.status);
        });
        ui.separator();
        let texture = if self.show_original {
            self.source_texture.as_ref()
        } else {
            self.preview_texture.as_ref()
        };
        if let Some(texture) = texture {
            show_texture(ui, texture);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Abre una imagen para mostrar la previsualización");
            });
        }
    }
}

fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(82.0, 0.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| ui.label(label),
        );
        let changed = ui
            .add_sized(
                egui::vec2(150.0, 20.0),
                egui::Slider::new(value, range).show_value(false),
            )
            .changed();
        ui.monospace(format!("{value:.2}"));
        changed
    })
    .inner
}

fn show_texture(ui: &mut egui::Ui, texture: &TextureHandle) {
    let available = ui.available_size();
    let size = texture.size_vec2();
    let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
    ui.image((texture.id(), size * scale));
}
