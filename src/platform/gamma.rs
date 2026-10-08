use super::{DisplayFilter, DisplayTarget};
use crate::{
    domain::ColorAdjustments,
    error::{AppError, AppResult},
};
use std::ptr;
use windows_sys::Win32::Graphics::Gdi::{
    CreateDCW, DISPLAY_DEVICEW, DeleteDC, EnumDisplayDevicesW,
};
use windows_sys::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

struct GammaTarget {
    device_name: Vec<u16>,
    label: String,
}

pub struct GammaDisplayFilter {
    targets: Vec<GammaTarget>,
    selected_target: usize,
    original_ramp: Option<(usize, [u16; 768])>,
    diagnostics: String,
}

impl GammaDisplayFilter {
    pub fn new() -> Self {
        let targets = enumerate_targets();
        tracing::info!(targets = targets.len(), "Gamma: salidas activas enumeradas");
        Self {
            targets,
            selected_target: 0,
            original_ramp: None,
            diagnostics: "Backend gamma de Windows. La saturación requiere AMD ADL o NVIDIA NVAPI."
                .to_string(),
        }
    }

    pub fn with_diagnostics(nvidia: &str, amd: &str) -> Self {
        Self {
            diagnostics: format!(
                "Gamma por monitor. Saturación no disponible en este backend. NVIDIA: {nvidia}. AMD: {amd}."
            ),
            ..Self::new()
        }
    }
}

impl DisplayFilter for GammaDisplayFilter {
    fn apply(&mut self, adjustments: ColorAdjustments) -> AppResult<()> {
        if !adjustments.is_valid() {
            return Err(AppError::InvalidAdjustments);
        }
        let target = self.targets.get(self.selected_target).ok_or_else(|| {
            AppError::DisplayFilterUnavailable(
                "no hay una pantalla activa seleccionada".to_string(),
            )
        })?;
        unsafe {
            let dc = CreateDCW(
                std::ptr::null(),
                target.device_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            );
            if dc.is_null() {
                tracing::error!(target = %target.label, "Gamma: CreateDCW falló");
                return Err(AppError::DisplayFilterUnavailable(format!(
                    "no se pudo acceder a {}",
                    target.label
                )));
            }
            if self.original_ramp.is_none() {
                let mut ramp = [0u16; 768];
                if GetDeviceGammaRamp(dc, ramp.as_mut_ptr().cast()) == 0 {
                    tracing::warn!(target = %target.label, "Gamma: GetDeviceGammaRamp falló");
                    DeleteDC(dc);
                    return Err(AppError::DisplayFilterUnavailable(format!(
                        "{} no permite modificar la gamma",
                        target.label
                    )));
                }
                self.original_ramp = Some((self.selected_target, ramp));
            }
            let ramp = build_ramp(adjustments);
            if SetDeviceGammaRamp(dc, ramp.as_ptr().cast()) == 0 {
                tracing::error!(target = %target.label, "Gamma: SetDeviceGammaRamp falló");
                DeleteDC(dc);
                return Err(AppError::DisplayFilterUnavailable(format!(
                    "no se pudo aplicar la gamma en {}",
                    target.label
                )));
            }
            DeleteDC(dc);
        }
        Ok(())
    }

    fn restore(&mut self) -> AppResult<()> {
        unsafe {
            if let Some((target_index, ramp)) = self.original_ramp.take()
                && let Some(target) = self.targets.get(target_index)
            {
                let dc = CreateDCW(
                    std::ptr::null(),
                    target.device_name.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                );
                if dc.is_null() || SetDeviceGammaRamp(dc, ramp.as_ptr().cast()) == 0 {
                    if !dc.is_null() {
                        DeleteDC(dc);
                    }
                    return Err(AppError::DisplayFilterUnavailable(format!(
                        "no se pudo restaurar la gamma en {}",
                        target.label
                    )));
                }
                DeleteDC(dc);
            }
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        !self.targets.is_empty()
    }

    fn backend_name(&self) -> &str {
        "Windows Gamma por monitor (fallback)"
    }

    fn targets(&self) -> Vec<DisplayTarget> {
        self.targets
            .iter()
            .map(|target| DisplayTarget {
                label: target.label.clone(),
            })
            .collect()
    }

    fn select_target(&mut self, index: usize) {
        if self.targets.get(index).is_some() && self.selected_target != index {
            let _ = self.restore();
            self.selected_target = index;
        }
    }

    fn diagnostics(&self) -> &str {
        &self.diagnostics
    }
}

fn enumerate_targets() -> Vec<GammaTarget> {
    let mut targets = Vec::new();
    for index in 0..32 {
        let mut device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        let result = unsafe { EnumDisplayDevicesW(ptr::null(), index, &mut device, 0) };
        if result == 0 || device.StateFlags & 0x00000001 == 0 {
            continue;
        }
        let end = device
            .DeviceName
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(device.DeviceName.len());
        let label_end = device
            .DeviceString
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(device.DeviceString.len());
        let name = String::from_utf16_lossy(&device.DeviceName[..end]);
        let adapter_description = String::from_utf16_lossy(&device.DeviceString[..label_end]);
        let monitor_description = monitor_description(&device.DeviceName);
        let mut device_name = device.DeviceName[..=end].to_vec();
        if device_name.last().copied() != Some(0) {
            device_name.push(0);
        }
        targets.push(GammaTarget {
            device_name,
            label: format!(
                "{} - {}",
                monitor_description
                    .as_deref()
                    .unwrap_or(adapter_description.trim()),
                name.trim_start_matches("\\\\.\\")
            ),
        });
    }

    fn monitor_description(adapter_name: &[u16]) -> Option<String> {
        let mut monitor = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        let result = unsafe { EnumDisplayDevicesW(adapter_name.as_ptr(), 0, &mut monitor, 0) };
        if result == 0 {
            return None;
        }
        let end = monitor
            .DeviceString
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(monitor.DeviceString.len());
        let description = String::from_utf16_lossy(&monitor.DeviceString[..end]);
        (!description.trim().is_empty()).then(|| description.trim().to_string())
    }
    targets
}

fn build_ramp(adjustments: ColorAdjustments) -> [u16; 768] {
    let mut ramp = [0u16; 768];
    for index in 0..256 {
        let input = index as f32 / 255.0;
        let value = ((input - 0.5) * adjustments.contrast + 0.5 + adjustments.brightness)
            .clamp(0.0, 1.0)
            .powf(1.0 / adjustments.gamma);
        ramp[index] = ((value + adjustments.temperature * 0.1).clamp(0.0, 1.0) * 65535.0) as u16;
        ramp[256 + index] = ((value + adjustments.tint * 0.05).clamp(0.0, 1.0) * 65535.0) as u16;
        ramp[512 + index] =
            ((value - adjustments.temperature * 0.1).clamp(0.0, 1.0) * 65535.0) as u16;
    }
    ramp
}
