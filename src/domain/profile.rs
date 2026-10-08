use serde::{Deserialize, Serialize};

use super::ColorAdjustments;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorProfile {
    pub version: u32,
    pub name: String,
    pub adjustments: ColorAdjustments,
}

impl ColorProfile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new(name: impl Into<String>, adjustments: ColorAdjustments) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            name: name.into(),
            adjustments,
        }
    }

    pub fn validate(&self) -> bool {
        self.version == Self::CURRENT_VERSION
            && !self.name.trim().is_empty()
            && self.adjustments.is_valid()
    }
}
