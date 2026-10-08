#[cfg(not(windows))]
use crate::error::AppError;
use crate::{domain::ColorAdjustments, error::AppResult};

mod hotkey;
pub use hotkey::{Hotkey, HotkeyManager};

#[cfg(windows)]
mod amd;
#[cfg(windows)]
mod gamma;
#[cfg(windows)]
mod nvidia;

pub trait DisplayFilter {
    fn apply(&mut self, adjustments: ColorAdjustments) -> AppResult<()>;
    fn restore(&mut self) -> AppResult<()>;
    fn is_available(&self) -> bool;
    fn backend_name(&self) -> &str;
    fn targets(&self) -> Vec<DisplayTarget>;
    fn select_target(&mut self, index: usize);
    fn diagnostics(&self) -> &str;
}

#[cfg(windows)]
pub fn detected_gpus() -> Vec<String> {
    use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

    let Ok(class_key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(
        r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
    ) else {
        return Vec::new();
    };

    (0..32)
        .filter_map(|index| {
            let key = class_key.open_subkey(format!("{index:04}")).ok()?;
            let name: String = key.get_value("DriverDesc").ok()?;
            (!name.trim().is_empty()).then_some(name)
        })
        .fold(Vec::new(), |mut names, name| {
            if !names.iter().any(|existing| existing == &name) {
                names.push(name);
            }
            names
        })
}

#[cfg(not(windows))]
pub fn detected_gpus() -> Vec<String> {
    Vec::new()
}

#[derive(Debug, Clone)]
pub struct DisplayTarget {
    pub label: String,
}

pub fn create_display_filter() -> Box<dyn DisplayFilter> {
    #[cfg(windows)]
    {
        tracing::info!("iniciando detección de backends de color");
        if let Some(filter) = nvidia::NvidiaDisplayFilter::load() {
            tracing::info!(
                backend = filter.backend_name(),
                "backend NVIDIA seleccionado"
            );
            return Box::new(filter);
        }
        tracing::warn!(
            reason = nvidia::diagnostics(),
            "backend NVIDIA no disponible"
        );
        if let Some(filter) = amd::AmdDisplayFilter::load() {
            tracing::info!(backend = filter.backend_name(), "backend AMD seleccionado");
            return Box::new(filter);
        }
        tracing::warn!(
            reason = amd::diagnostics(),
            "backend AMD no disponible; usando fallback"
        );
        Box::new(gamma::GammaDisplayFilter::with_diagnostics(
            nvidia::diagnostics(),
            amd::diagnostics(),
        ))
    }
    #[cfg(not(windows))]
    {
        Box::new(UnsupportedDisplayFilter)
    }
}

#[cfg(not(windows))]
struct UnsupportedDisplayFilter;

#[cfg(not(windows))]
impl DisplayFilter for UnsupportedDisplayFilter {
    fn apply(&mut self, _adjustments: ColorAdjustments) -> AppResult<()> {
        Err(AppError::DisplayFilterUnavailable(
            "esta plataforma todavía no tiene un backend de pantalla".to_string(),
        ))
    }

    fn restore(&mut self) -> AppResult<()> {
        Ok(())
    }

    fn is_available(&self) -> bool {
        false
    }

    fn backend_name(&self) -> &str {
        "No compatible display backend"
    }

    fn targets(&self) -> Vec<DisplayTarget> {
        Vec::new()
    }

    fn select_target(&mut self, _index: usize) {}

    fn diagnostics(&self) -> &str {
        "La plataforma no es Windows"
    }
}
