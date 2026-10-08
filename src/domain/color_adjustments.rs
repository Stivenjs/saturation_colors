use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorAdjustments {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
    pub gamma: f32,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            gamma: 1.0,
        }
    }
}

impl ColorAdjustments {
    pub const BRIGHTNESS_RANGE: std::ops::RangeInclusive<f32> = -1.0..=1.0;
    pub const CONTRAST_RANGE: std::ops::RangeInclusive<f32> = 0.0..=2.0;
    pub const SATURATION_RANGE: std::ops::RangeInclusive<f32> = 0.0..=2.0;
    pub const TEMPERATURE_RANGE: std::ops::RangeInclusive<f32> = -1.0..=1.0;
    pub const TINT_RANGE: std::ops::RangeInclusive<f32> = -1.0..=1.0;
    pub const GAMMA_RANGE: std::ops::RangeInclusive<f32> = 0.1..=4.0;

    pub fn is_valid(self) -> bool {
        self.brightness.is_finite()
            && self.contrast.is_finite()
            && self.saturation.is_finite()
            && self.temperature.is_finite()
            && self.tint.is_finite()
            && self.gamma.is_finite()
            && Self::BRIGHTNESS_RANGE.contains(&self.brightness)
            && Self::CONTRAST_RANGE.contains(&self.contrast)
            && Self::SATURATION_RANGE.contains(&self.saturation)
            && Self::TEMPERATURE_RANGE.contains(&self.temperature)
            && Self::TINT_RANGE.contains(&self.tint)
            && Self::GAMMA_RANGE.contains(&self.gamma)
    }
}

#[cfg(test)]
mod tests {
    use super::ColorAdjustments;

    #[test]
    fn defaults_are_valid() {
        assert!(ColorAdjustments::default().is_valid());
    }

    #[test]
    fn values_outside_ranges_are_rejected() {
        let invalid = ColorAdjustments {
            gamma: 0.0,
            ..Default::default()
        };
        assert!(!invalid.is_valid());
    }
}
