use super::{DisplayFilter, DisplayTarget};
use crate::{
    domain::ColorAdjustments,
    error::{AppError, AppResult},
};
use libloading::Library;
use std::sync::OnceLock;

type MemoryAlloc = unsafe extern "C" fn(i32) -> *mut core::ffi::c_void;
type MainCreate = unsafe extern "C" fn(MemoryAlloc, i32) -> i32;
type MainDestroy = unsafe extern "C" fn() -> i32;
type AdapterCount = unsafe extern "C" fn(*mut i32) -> i32;
type AdapterInfoGet = unsafe extern "C" fn(*mut core::ffi::c_void, i32) -> i32;
type AdapterActiveGet = unsafe extern "C" fn(i32, *mut i32) -> i32;
type DisplayInfoGet = unsafe extern "C" fn(i32, *mut i32, *mut *mut core::ffi::c_void, i32) -> i32;
type ColorSet = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
type ColorGet =
    unsafe extern "C" fn(i32, i32, i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> i32;

const ADL_DISPLAY_COLOR_SATURATION: i32 = 1 << 2;
const ADL_MAX_PATH: usize = 256;
const ADL_MAX_ADAPTERS: usize = 40;
const ADL_DISPLAY_LOGICAL_ADAPTER_INDEX_INVALID: i32 = -1;
static DIAGNOSTICS: OnceLock<String> = OnceLock::new();

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlAdapterInfo {
    size: i32,
    adapter_index: i32,
    udid: [i8; ADL_MAX_PATH],
    bus_number: i32,
    driver_number: i32,
    function_number: i32,
    vendor_id: i32,
    adapter_name: [i8; ADL_MAX_PATH],
    display_name: [i8; ADL_MAX_PATH],
    present: i32,
    exist: i32,
    driver_path: [i8; ADL_MAX_PATH],
    driver_path_ext: [i8; ADL_MAX_PATH],
    pnp_string: [i8; ADL_MAX_PATH],
    os_display_index: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlDisplayId {
    logical_index: i32,
    physical_index: i32,
    logical_adapter_index: i32,
    physical_adapter_index: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AdlDisplayInfo {
    display_id: AdlDisplayId,
    display_controller_index: i32,
    display_name: [i8; ADL_MAX_PATH],
    manufacturer_name: [i8; ADL_MAX_PATH],
    display_type: i32,
    output_type: i32,
    connector: i32,
    info_mask: i32,
    info_value: i32,
}

pub fn diagnostics() -> &'static str {
    DIAGNOSTICS
        .get_or_init(|| "ADL no se ha inicializado.".to_string())
        .as_str()
}

unsafe extern "C" fn adl_memory_alloc(size: i32) -> *mut core::ffi::c_void {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    unsafe { windows_sys::Win32::System::Com::CoTaskMemAlloc(size as usize) }
}

pub struct AmdDisplayFilter {
    _library: Library,
    destroy: MainDestroy,
    color_set: ColorSet,
    color_get: ColorGet,
    targets: Vec<DisplayTarget>,
    target_handles: Vec<(i32, i32)>,
    selected_target: Option<usize>,
    original: Vec<(i32, i32, i32)>,
}

impl AmdDisplayFilter {
    pub fn load() -> Option<Self> {
        tracing::info!("AMD: intentando cargar atiadlxx.dll");
        let library = unsafe {
            Library::new("atiadlxx.dll")
                .or_else(|error| {
                    tracing::debug!(%error, "AMD: atiadlxx.dll no se pudo cargar; probando atiadlxy.dll");
                    Library::new("atiadlxy.dll")
                })
                .map_err(|error| {
                    let _ = DIAGNOSTICS.set(format!("DLL ADL no encontrada: {error}"));
                    tracing::debug!(%error, "no se encontró la DLL de AMD ADL");
                    error
                })
                .ok()?
        };
        tracing::info!("AMD: DLL ADL cargada");
        unsafe {
            let create: MainCreate = match library.get(b"ADL_Main_Control_Create\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Main_Control_Create: {error}"));
                    tracing::debug!(%error, "ADL_Main_Control_Create no está disponible");
                    return None;
                }
            };
            let destroy: MainDestroy = match library.get(b"ADL_Main_Control_Destroy\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Main_Control_Destroy: {error}"));
                    tracing::debug!(%error, "ADL_Main_Control_Destroy no está disponible");
                    return None;
                }
            };
            let get_count: AdapterCount = match library.get(b"ADL_Adapter_NumberOfAdapters_Get\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ =
                        DIAGNOSTICS.set(format!("Falta ADL_Adapter_NumberOfAdapters_Get: {error}"));
                    tracing::debug!(%error, "ADL_Adapter_NumberOfAdapters_Get no está disponible");
                    return None;
                }
            };
            let adapter_info_get: AdapterInfoGet = match library
                .get(b"ADL_Adapter_AdapterInfo_Get\0")
            {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Adapter_AdapterInfo_Get: {error}"));
                    tracing::debug!(%error, "ADL_Adapter_AdapterInfo_Get no está disponible");
                    return None;
                }
            };
            let adapter_active_get: AdapterActiveGet =
                match library.get(b"ADL_Adapter_Active_Get\0") {
                    Ok(symbol) => *symbol,
                    Err(error) => {
                        let _ = DIAGNOSTICS.set(format!("Falta ADL_Adapter_Active_Get: {error}"));
                        tracing::debug!(%error, "ADL_Adapter_Active_Get no está disponible");
                        return None;
                    }
                };
            let display_info_get: DisplayInfoGet = match library
                .get(b"ADL_Display_DisplayInfo_Get\0")
            {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Display_DisplayInfo_Get: {error}"));
                    tracing::debug!(%error, "ADL_Display_DisplayInfo_Get no está disponible");
                    return None;
                }
            };
            let color_set: ColorSet = match library.get(b"ADL_Display_Color_Set\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Display_Color_Set: {error}"));
                    tracing::debug!(%error, "ADL_Display_Color_Set no está disponible");
                    return None;
                }
            };
            let color_get: ColorGet = match library.get(b"ADL_Display_Color_Get\0") {
                Ok(symbol) => *symbol,
                Err(error) => {
                    let _ = DIAGNOSTICS.set(format!("Falta ADL_Display_Color_Get: {error}"));
                    tracing::debug!(%error, "ADL_Display_Color_Get no está disponible");
                    return None;
                }
            };
            let create_status = create(adl_memory_alloc, 1);
            tracing::info!(
                status = create_status,
                "AMD: ADL_Main_Control_Create ejecutado"
            );
            if create_status != 0 {
                let _ =
                    DIAGNOSTICS.set(format!("ADL_Main_Control_Create devolvió {create_status}"));
                tracing::debug!(status = create_status, "ADL no pudo inicializarse");
                return None;
            }
            let mut adapter_count = 0;
            let count_status = get_count(&mut adapter_count);
            tracing::info!(
                status = count_status,
                adapter_count,
                "AMD: adaptadores enumerados"
            );
            if count_status != 0 || adapter_count <= 0 {
                let _ = DIAGNOSTICS.set("ADL no encontró adaptadores AMD.".to_string());
                tracing::debug!(adapter_count, "ADL no detectó adaptadores AMD");
                let _ = destroy();
                return None;
            }
            let mut adapter_info = vec![
                AdlAdapterInfo {
                    size: std::mem::size_of::<AdlAdapterInfo>() as i32,
                    adapter_index: 0,
                    udid: [0; ADL_MAX_PATH],
                    bus_number: 0,
                    driver_number: 0,
                    function_number: 0,
                    vendor_id: 0,
                    adapter_name: [0; ADL_MAX_PATH],
                    display_name: [0; ADL_MAX_PATH],
                    present: 0,
                    exist: 0,
                    driver_path: [0; ADL_MAX_PATH],
                    driver_path_ext: [0; ADL_MAX_PATH],
                    pnp_string: [0; ADL_MAX_PATH],
                    os_display_index: 0,
                };
                ADL_MAX_ADAPTERS
            ];
            let adapter_info_status = adapter_info_get(
                adapter_info.as_mut_ptr().cast(),
                (std::mem::size_of::<AdlAdapterInfo>() * adapter_info.len()) as i32,
            );
            tracing::info!(
                status = adapter_info_status,
                "AMD: información de adaptadores obtenida"
            );
            if adapter_info_status != 0 {
                let _ = DIAGNOSTICS.set(format!(
                    "ADL_Adapter_AdapterInfo_Get devolvió {adapter_info_status}"
                ));
                let _ = destroy();
                return None;
            }
            let mut targets = Vec::new();
            let mut target_handles = Vec::new();
            for info in adapter_info.iter().take(adapter_count as usize) {
                let adapter = info.adapter_index;
                let mut active = 0;
                let active_status = adapter_active_get(adapter, &mut active);
                tracing::debug!(
                    adapter,
                    active,
                    status = active_status,
                    "AMD: estado de adaptador"
                );
                if active_status != 0 || active == 0 {
                    continue;
                }
                let mut display_count = 0;
                let mut display_buffer = std::ptr::null_mut();
                let display_status =
                    display_info_get(adapter, &mut display_count, &mut display_buffer, 1);
                tracing::debug!(
                    adapter,
                    display_count,
                    status = display_status,
                    "AMD: displays obtenidos"
                );
                if display_status != 0 || display_buffer.is_null() {
                    continue;
                }
                let display_size = std::mem::size_of::<AdlDisplayInfo>();
                for display_offset in 0..display_count.max(0) as usize {
                    let display_info = std::ptr::read(
                        (display_buffer.cast::<u8>()).add(display_offset * display_size)
                            as *const AdlDisplayInfo,
                    );
                    let display_index = display_info.display_id.logical_index;
                    if display_info.display_id.logical_adapter_index
                        == ADL_DISPLAY_LOGICAL_ADAPTER_INDEX_INVALID
                    {
                        continue;
                    }
                    let (mut current, mut default, mut minimum, mut maximum, mut step) =
                        (0, 0, 0, 0, 0);
                    let color_status = color_get(
                        adapter,
                        display_index,
                        ADL_DISPLAY_COLOR_SATURATION,
                        &mut current,
                        &mut default,
                        &mut minimum,
                        &mut maximum,
                        &mut step,
                    );
                    if color_status == 0 {
                        tracing::debug!(
                            adapter,
                            display_index,
                            current,
                            default,
                            minimum,
                            maximum,
                            step,
                            "AMD: display con saturación disponible"
                        );
                        targets.push(DisplayTarget {
                            label: format!(
                                "AMD - {} / {}",
                                c_string(&info.display_name),
                                c_string(&display_info.display_name)
                            ),
                        });
                        target_handles.push((adapter, display_index));
                    } else if color_status != -1 {
                        tracing::trace!(
                            adapter,
                            display_index,
                            status = color_status,
                            "AMD: display rechazado"
                        );
                    }
                }
                windows_sys::Win32::System::Com::CoTaskMemFree(display_buffer);
            }
            tracing::info!(
                targets = targets.len(),
                "AMD: displays controlables encontrados"
            );
            if targets.is_empty() {
                let _ = DIAGNOSTICS
                    .set("ADL encontró AMD, pero ningún monitor controlable.".to_string());
                tracing::debug!("ADL no encontró displays AMD con control de saturación");
                let _ = destroy();
                return None;
            }
            Some(Self {
                _library: library,
                destroy,
                color_set,
                color_get,
                targets,
                target_handles,
                selected_target: Some(0),
                original: Vec::new(),
            })
        }
    }
}

impl DisplayFilter for AmdDisplayFilter {
    fn apply(&mut self, adjustments: ColorAdjustments) -> AppResult<()> {
        let saturation = (adjustments.saturation * 100.0).round() as i32;
        tracing::debug!(saturation, target = ?self.selected_target, "AMD: aplicando saturación");
        unsafe {
            if self.original.is_empty() {
                let Some(target_index) = self.selected_target else {
                    return Err(AppError::DisplayFilterUnavailable(
                        "no hay un monitor AMD seleccionado".to_string(),
                    ));
                };
                let Some(&(adapter, display)) = self.target_handles.get(target_index) else {
                    return Err(AppError::DisplayFilterUnavailable(
                        "el monitor AMD seleccionado ya no está disponible".to_string(),
                    ));
                };
                let (mut current, mut default, mut minimum, mut maximum, mut step) =
                    (0, 0, 0, 0, 0);
                if (self.color_get)(
                    adapter,
                    display,
                    ADL_DISPLAY_COLOR_SATURATION,
                    &mut current,
                    &mut default,
                    &mut minimum,
                    &mut maximum,
                    &mut step,
                ) == 0
                {
                    self.original.push((adapter, display, current));
                }
            }
            if self.original.is_empty() {
                return Err(AppError::DisplayFilterUnavailable(
                    "ADL no encontró pantallas AMD controlables".to_string(),
                ));
            }
            for &(adapter, display, _) in &self.original {
                if (self.color_set)(
                    adapter,
                    display,
                    ADL_DISPLAY_COLOR_SATURATION,
                    saturation.clamp(0, 200),
                ) != 0
                {
                    return Err(AppError::DisplayFilterUnavailable(
                        "ADL no pudo cambiar la saturación".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn restore(&mut self) -> AppResult<()> {
        tracing::debug!(
            entries = self.original.len(),
            "AMD: restaurando valores originales"
        );
        unsafe {
            for &(adapter, display, value) in &self.original {
                if (self.color_set)(adapter, display, ADL_DISPLAY_COLOR_SATURATION, value) != 0 {
                    return Err(AppError::DisplayFilterUnavailable(
                        "ADL no pudo restaurar la saturación".to_string(),
                    ));
                }
            }
        }
        self.original.clear();
        Ok(())
    }

    fn is_available(&self) -> bool {
        !self.targets.is_empty()
    }

    fn backend_name(&self) -> &str {
        "AMD ADL"
    }

    fn targets(&self) -> Vec<DisplayTarget> {
        self.targets.clone()
    }

    fn select_target(&mut self, index: usize) {
        if self.targets.get(index).is_some() && self.selected_target != Some(index) {
            let _ = self.restore();
            self.selected_target = Some(index);
        }
    }

    fn diagnostics(&self) -> &str {
        "ADL inicializado correctamente."
    }
}

impl Drop for AmdDisplayFilter {
    fn drop(&mut self) {
        let _ = self.restore();
        unsafe {
            let _ = (self.destroy)();
        }
    }
}

fn c_string(value: &[i8]) -> String {
    let bytes = value
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).trim().to_string()
}
