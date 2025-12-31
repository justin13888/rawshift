//! Color space transformations.
//!
//! This transform module orchestrates the color processing pipeline, handling:
//! - White Balance application
//! - Color Space conversion (Camera RGB -> Output RGB)
//! - Gamma Correction
//!
//! It utilizes the optimized primitives from [`crate::processing::color`].

use crate::core::image::RgbImage;
use crate::error::RawResult;
use crate::processing::color::{apply_color_matrix, apply_gamma, apply_white_balance};

/// Matrix to convert from CIE XYZ to sRGB (D65).
/// (Simplified for example purposes).
// TODO: Replace this. Calculate and verify
pub const XYZ_TO_SRGB_MATRIX: [f32; 9] = [
    3.2406, -1.5372, -0.4986, -0.9689, 1.8758, 0.0415, 0.0557, -0.2040, 1.0570,
];

/// Pipeline step for color space corrections.
pub struct ColorSpaceTransform {
    /// White balance multipliers (R, G, B)
    pub wb_coeffs: (f32, f32, f32),
    /// Color matrix (Camera RGB -> Output RGB)
    /// This should be the Pre-calculated product of:
    /// XYZ->Output * Camera->XYZ
    pub color_matrix: [f32; 9],
    /// Gamma value for final encoding
    pub gamma: f32,
}

impl ColorSpaceTransform {
    /// Create a new color transform with specific settings.
    pub fn new(wb_coeffs: (f32, f32, f32), color_matrix: [f32; 9], gamma: f32) -> Self {
        Self {
            wb_coeffs,
            color_matrix,
            gamma,
        }
    }

    /// Apply the color transformation pipeline to an image in-place.
    pub fn apply(&self, image: &mut RgbImage) -> RawResult<()> {
        // 1. Apply White Balance (Linear -> Linear)
        apply_white_balance(image, self.wb_coeffs);

        // 2. Apply Color Matrix (Camera Linear -> Output Linear)
        apply_color_matrix(image, &self.color_matrix);

        // 3. Apply Gamma Correction (Output Linear -> Output Encoded)
        apply_gamma(image, self.gamma);

        Ok(())
    }
}

// TODO: Flesh out this module
