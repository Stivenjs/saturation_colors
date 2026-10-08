use std::path::Path;

use crate::{
    domain::ColorProfile,
    error::{AppError, AppResult},
};

pub fn save_profile(path: &Path, profile: &ColorProfile) -> AppResult<()> {
    if !profile.validate() {
        return Err(AppError::InvalidAdjustments);
    }
    let json = serde_json::to_string_pretty(profile)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_profile(path: &Path) -> AppResult<ColorProfile> {
    let profile: ColorProfile = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    if !profile.validate() {
        return Err(AppError::InvalidAdjustments);
    }
    Ok(profile)
}
