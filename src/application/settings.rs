use crate::domain::ColorAdjustments;

fn settings_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "Saturation Colors", "Saturation Colors")
        .map(|directories| directories.config_dir().join("adjustments.json"))
}

pub fn load_adjustments() -> ColorAdjustments {
    let Some(path) = settings_path() else {
        return ColorAdjustments::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(adjustments) => adjustments,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "ajustes guardados inválidos");
                ColorAdjustments::default()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ColorAdjustments::default(),
        Err(error) => {
            tracing::warn!(%error, path = %path.display(), "no se pudieron leer los ajustes guardados");
            ColorAdjustments::default()
        }
    }
}

pub fn save_adjustments(adjustments: ColorAdjustments) -> Result<(), String> {
    if !adjustments.is_valid() {
        return Err("no se pueden guardar ajustes inválidos".to_string());
    }
    let path = settings_path()
        .ok_or_else(|| "no se pudo determinar la carpeta de configuración".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "la ruta de configuración no tiene carpeta padre".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let contents = serde_json::to_string_pretty(&adjustments).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}
