use image::RgbaImage;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ImageData {
    pub image: Arc<RgbaImage>,
    pub source_name: String,
}

impl ImageData {
    pub fn new(image: RgbaImage, source_name: impl Into<String>) -> Self {
        Self {
            image: Arc::new(image),
            source_name: source_name.into(),
        }
    }
}
