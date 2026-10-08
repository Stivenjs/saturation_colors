use super::{DisplayFilter, DisplayTarget};
use crate::{
    domain::ColorAdjustments,
    error::{AppError, AppResult},
};
use libloading::Library;
use std::sync::OnceLock;

type QueryInterface = unsafe extern "C" fn(u32) -> *const core::ffi::c_void;
type Initialize = unsafe extern "C" fn() -> i32;
type EnumDisplayHandle = unsafe extern "C" fn(u32, *mut *mut core::ffi::c_void) -> i32;
type GetDvcInfo = unsafe extern "C" fn(*mut core::ffi::c_void, *mut NvDisplayDvcInfo) -> i32;
type SetDvcLevel = unsafe extern "C" fn(*mut core::ffi::c_void, i32) -> i32;

#[repr(C)]
struct NvDisplayDvcInfo {
    version: u32,
    current_level: i32,
    min_level: i32,
    max_level: i32,
}

pub struct NvidiaDisplayFilter {
    _library: Library,
    displays: Vec<*mut core::ffi::c_void>,
    get_info: GetDvcInfo,
    set_level: SetDvcLevel,
    original: Vec<(i32, i32, i32)>,
}

static DIAGNOSTICS: OnceLock<String> = OnceLock::new();

pub fn diagnostics() -> &'static str {
    DIAGNOSTICS
        .get_or_init(|| {
            "NVAPI desactivado por seguridad. Activa SATURATION_COLORS_ENABLE_NVAPI=1 solo para pruebas."
                .to_string()
        })
        .as_str()
}

impl NvidiaDisplayFilter {
    pub fn load() -> Option<Self> {
        // NVAPI exposes an undocumented C ABI through QueryInterface. Keep it
        // opt-in until the exact driver ABI has been validated on the target
        // machine; an ABI mismatch terminates the process with an access
        // violation and cannot be handled as a Rust Result.
        if std::env::var_os("SATURATION_COLORS_ENABLE_NVAPI").is_none() {
            let _ = DIAGNOSTICS
                .set("NVAPI desactivado por seguridad; no se enumeran salidas NVIDIA.".to_string());
            tracing::info!(
                "NVAPI está disponible como backend experimental; usando gamma ramp por defecto"
            );
            return None;
        }
        tracing::info!("NVIDIA: NVAPI habilitado explícitamente; cargando nvapi64.dll");
        let library = unsafe {
            Library::new("nvapi64.dll")
                .map_err(|error| {
                    let _ = DIAGNOSTICS.set(format!("DLL NVAPI no encontrada: {error}"));
                    error
                })
                .ok()?
        };
        tracing::info!("NVIDIA: nvapi64.dll cargada");
        unsafe {
            let query: QueryInterface = match library.get(b"nvapi_QueryInterface\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta nvapi_QueryInterface: {error}"));
                    tracing::error!(%error, "NVIDIA: falta nvapi_QueryInterface");
                    return None;
                }
            };
            tracing::debug!("NVIDIA: nvapi_QueryInterface encontrado");
            let initialize = query_function::<Initialize>(&query, 0x0150E828)?;
            let initialize_status = initialize();
            tracing::info!(
                status = initialize_status,
                "NVIDIA: NvAPI_Initialize ejecutado"
            );
            if initialize_status != 0 {
                return None;
            }
            let enum_display = query_function::<EnumDisplayHandle>(&query, 0x9ABDD40D)?;
            let get_info = query_function::<GetDvcInfo>(&query, 0x4085DE45)?;
            let set_level = query_function::<SetDvcLevel>(&query, 0x172409B4)?;
            let mut displays = Vec::new();
            for index in 0..16 {
                let mut handle = std::ptr::null_mut();
                if enum_display(index, &mut handle) != 0 {
                    tracing::debug!(index, "NVIDIA: fin de enumeración de displays");
                    break;
                }
                displays.push(handle);
            }
            tracing::info!(displays = displays.len(), "NVIDIA: displays enumerados");
            if displays.is_empty() {
                return None;
            }
            Some(Self {
                _library: library,
                displays,
                get_info,
                set_level,
                original: Vec::new(),
            })
        }
    }
}

unsafe fn query_function<T>(query: &QueryInterface, id: u32) -> Option<T> {
    let pointer = unsafe { query(id) };
    (!pointer.is_null()).then(|| unsafe { pointer.cast::<T>().read() })
}

impl DisplayFilter for NvidiaDisplayFilter {
    fn apply(&mut self, adjustments: ColorAdjustments) -> AppResult<()> {
        tracing::debug!(
            saturation = adjustments.saturation,
            "NVIDIA: aplicando Digital Vibrance"
        );
        unsafe {
            if self.original.is_empty() {
                for (index, &display) in self.displays.iter().enumerate() {
                    let mut info = NvDisplayDvcInfo {
                        version: std::mem::size_of::<NvDisplayDvcInfo>() as u32 | (1 << 16),
                        current_level: 0,
                        min_level: 0,
                        max_level: 0,
                    };
                    if (self.get_info)(display, &mut info) == 0 {
                        self.original
                            .push((index as i32, info.current_level, info.max_level));
                    }
                }
            }
            if self.original.is_empty() {
                return Err(AppError::DisplayFilterUnavailable(
                    "NVAPI no expone Digital Vibrance en estas salidas".to_string(),
                ));
            }
            for &(index, _, max_level) in &self.original {
                let level = (adjustments.saturation * max_level as f32).round() as i32;
                if (self.set_level)(self.displays[index as usize], level.clamp(0, max_level)) != 0 {
                    return Err(AppError::DisplayFilterUnavailable(
                        "NVAPI no pudo cambiar Digital Vibrance".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn restore(&mut self) -> AppResult<()> {
        tracing::debug!(
            entries = self.original.len(),
            "NVIDIA: restaurando Digital Vibrance"
        );
        unsafe {
            for &(index, value, _) in &self.original {
                if (self.set_level)(self.displays[index as usize], value) != 0 {
                    return Err(AppError::DisplayFilterUnavailable(
                        "NVAPI no pudo restaurar Digital Vibrance".to_string(),
                    ));
                }
            }
        }
        self.original.clear();
        Ok(())
    }

    fn is_available(&self) -> bool {
        !self.displays.is_empty()
    }

    fn backend_name(&self) -> &str {
        "NVIDIA NVAPI (experimental)"
    }

    fn targets(&self) -> Vec<DisplayTarget> {
        self.displays
            .iter()
            .enumerate()
            .map(|(index, _)| DisplayTarget {
                label: format!("NVIDIA - salida {index}"),
            })
            .collect()
    }

    fn select_target(&mut self, _index: usize) {}

    fn diagnostics(&self) -> &str {
        "NVAPI inicializado correctamente."
    }
}
