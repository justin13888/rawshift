//! Color space transformations.
//!
//! This transform module provides the canonical entry points for color processing:
//! - White Balance application
//! - Color Matrix application (Camera RGB -> Output RGB)
//!
//! It re-exports the optimized primitives from [`crate::processing::color`] and
//! provides the [`ColorSpaceTransform`] struct for bundled pipeline steps.

use crate::core::image::RgbImage;
use crate::error::RawResult;

// Re-export processing primitives as canonical transform entry points.
pub use crate::processing::color::{
    apply_color_matrix, apply_white_balance, apply_white_balance_raw,
};

/// Matrix to convert from CIE XYZ to sRGB (D65).
// TODO: Replace this. Calculate and verify
pub const XYZ_TO_SRGB_MATRIX: [f32; 9] = [
    3.2406, -1.5372, -0.4986, -0.9689, 1.8758, 0.0415, 0.0557, -0.2040, 1.0570,
];

/// Pipeline step for color space corrections.
///
/// Bundles white balance and color matrix into a single transform step.
/// Tone mapping / gamma is handled separately by [`crate::transforms::tonemap`].
pub struct ColorSpaceTransform {
    /// White balance multipliers (R, G, B)
    pub wb_coeffs: (f32, f32, f32),
    /// Color matrix (Camera RGB -> Output RGB)
    /// This should be the pre-calculated product of:
    /// XYZ->Output * Camera->XYZ
    pub color_matrix: [f32; 9],
}

impl ColorSpaceTransform {
    /// Create a new color transform with specific settings.
    pub fn new(wb_coeffs: (f32, f32, f32), color_matrix: [f32; 9]) -> Self {
        Self {
            wb_coeffs,
            color_matrix,
        }
    }

    /// Apply the color transformation pipeline to an image in-place.
    ///
    /// 1. Apply White Balance (Linear -> Linear)
    /// 2. Apply Color Matrix (Camera Linear -> Output Linear)
    pub fn apply(&self, image: &mut RgbImage) -> RawResult<()> {
        apply_white_balance(image, self.wb_coeffs);
        apply_color_matrix(image, &self.color_matrix);
        Ok(())
    }
}
