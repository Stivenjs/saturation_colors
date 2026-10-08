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
type GetDvcInfo = unsafe extern "C" fn(*mut core::ffi::c_void, u32, *mut NvDisplayDvcInfo) -> i32;
type SetDvcLevel = unsafe extern "C" fn(*mut core::ffi::c_void, u32, i32) -> i32;

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
        .get_or_init(|| "NVAPI no se ha inicializado.".to_string())
        .as_str()
}

impl NvidiaDisplayFilter {
    pub fn load() -> Option<Self> {
        // NVAPI is implemented by the installed NVIDIA driver. The DLL must
        // be loaded at runtime so systems without an NVIDIA driver can still
        // use the application and the gamma fallback.
        tracing::info!("NVIDIA: cargando nvapi64.dll");
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
                let _ = DIAGNOSTICS.set(format!(
                    "NvAPI_Initialize falló con código {initialize_status}"
                ));
                return None;
            }
            let Some(enum_display) = query_function::<EnumDisplayHandle>(&query, 0x9ABDD40D) else {
                let _ =
                    DIAGNOSTICS.set("NVAPI no expone NvAPI_EnumNvidiaDisplayHandle".to_string());
                return None;
            };
            let Some(get_info) = query_function::<GetDvcInfo>(&query, 0x4085DE45) else {
                let _ =
                    DIAGNOSTICS.set("NVAPI no expone la consulta de Digital Vibrance".to_string());
                return None;
            };
            let Some(set_level) = query_function::<SetDvcLevel>(&query, 0x172409B4) else {
                let _ =
                    DIAGNOSTICS.set("NVAPI no expone el ajuste de Digital Vibrance".to_string());
                return None;
            };
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
                let _ = DIAGNOSTICS.set(
                    "NVAPI se inicializó, pero no encontró salidas NVIDIA activas".to_string(),
                );
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
    if pointer.is_null() {
        return None;
    }

    // QueryInterface returns the function address itself. Reading from that
    // address would interpret the machine code bytes as a function pointer.
    // Reinterpret the pointer value instead.
    Some(unsafe { std::mem::transmute_copy(&pointer) })
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
                    if (self.get_info)(display, 0, &mut info) == 0 {
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
                let status =
                    (self.set_level)(self.displays[index as usize], 0, level.clamp(0, max_level));
                tracing::debug!(
                    index,
                    level,
                    status,
                    "NVIDIA: resultado de Digital Vibrance"
                );
                if status != 0 {
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
                let status = (self.set_level)(self.displays[index as usize], 0, value);
                tracing::debug!(
                    index,
                    value,
                    status,
                    "NVIDIA: restauración de Digital Vibrance"
                );
                if status != 0 {
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
