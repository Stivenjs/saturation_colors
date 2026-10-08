use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("no se pudo abrir la imagen: {0}")]
    ImageLoad(#[from] image::ImageError),
    #[error("no se pudo leer el perfil: {0}")]
    ProfileRead(#[from] std::io::Error),
    #[error("el perfil no tiene un formato válido: {0}")]
    InvalidProfile(#[from] serde_json::Error),
    #[error("el formato de imagen no es compatible")]
    UnsupportedFormat,
    #[error("los ajustes de color están fuera de rango")]
    InvalidAdjustments,
    #[error("los filtros globales no están disponibles: {0}")]
    DisplayFilterUnavailable(String),
}

pub type AppResult<T> = Result<T, AppError>;
