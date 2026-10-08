use std::path::Path;

use image::RgbaImage;

use crate::{
    domain::{ColorAdjustments, ImageData},
    error::{AppError, AppResult},
};

pub fn load_image(path: &Path) -> AppResult<ImageData> {
    let image = image::open(path)?.to_rgba8();
    let source_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(AppError::UnsupportedFormat)?;
    Ok(ImageData::new(image, source_name))
}

pub fn apply_adjustments(image: &RgbaImage, adjustments: ColorAdjustments) -> AppResult<RgbaImage> {
    if !adjustments.is_valid() {
        return Err(AppError::InvalidAdjustments);
    }

    let mut output = image.clone();
    for pixel in output.pixels_mut() {
        let [red, green, blue, alpha] = pixel.0;
        let (mut red, mut green, mut blue) = (
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
        );

        red = (red - 0.5) * adjustments.contrast + 0.5 + adjustments.brightness;
        green = (green - 0.5) * adjustments.contrast + 0.5 + adjustments.brightness;
        blue = (blue - 0.5) * adjustments.contrast + 0.5 + adjustments.brightness;

        red += adjustments.temperature * 0.1;
        blue -= adjustments.temperature * 0.1;
        green += adjustments.tint * 0.05;

        let (hue, saturation, value) = rgb_to_hsv(red, green, blue);
        let (red, green, blue) = hsv_to_rgb(
            hue,
            (saturation * adjustments.saturation).clamp(0.0, 1.0),
            value,
        );

        pixel.0 = [
            gamma_correct(red, adjustments.gamma),
            gamma_correct(green, adjustments.gamma),
            gamma_correct(blue, adjustments.gamma),
            alpha,
        ];
    }
    Ok(output)
}

fn gamma_correct(value: f32, gamma: f32) -> u8 {
    (value.clamp(0.0, 1.0).powf(1.0 / gamma) * 255.0).round() as u8
}

fn rgb_to_hsv(red: f32, green: f32, blue: f32) -> (f32, f32, f32) {
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == red {
        60.0 * (((green - blue) / delta) % 6.0)
    } else if max == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    (hue.rem_euclid(360.0), saturation, max)
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> (f32, f32, f32) {
    let chroma = value * saturation;
    let x = chroma * (1.0 - (((hue / 60.0) % 2.0) - 1.0).abs());
    let match_value = value - chroma;
    let (red, green, blue) = match hue {
        h if h < 60.0 => (chroma, x, 0.0),
        h if h < 120.0 => (x, chroma, 0.0),
        h if h < 180.0 => (0.0, chroma, x),
        h if h < 240.0 => (0.0, x, chroma),
        h if h < 300.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    (red + match_value, green + match_value, blue + match_value)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::apply_adjustments;
    use crate::domain::ColorAdjustments;

    #[test]
    fn defaults_preserve_pixels() {
        let image = RgbaImage::from_pixel(1, 1, Rgba([20, 40, 80, 128]));
        let processed = apply_adjustments(&image, ColorAdjustments::default()).unwrap();
        assert_eq!(processed, image);
    }

    #[test]
    fn alpha_is_preserved() {
        let image = RgbaImage::from_pixel(1, 1, Rgba([20, 40, 80, 37]));
        let processed = apply_adjustments(
            &image,
            ColorAdjustments {
                brightness: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(processed.get_pixel(0, 0)[3], 37);
    }
}
