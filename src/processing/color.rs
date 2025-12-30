use crate::core::image::RgbImage;

/// Apply white balance to an RGB image.
///
/// Multiplies each channel by the corresponding coefficient.
/// Data is clamped to u16 range.
pub fn apply_white_balance(image: &mut RgbImage, coeffs: (f32, f32, f32)) {
    let (r_scale, g_scale, b_scale) = coeffs;

    // Process pixel triplets
    for chunk in image.data.chunks_exact_mut(3) {
        // Red
        let r = chunk[0] as f32 * r_scale;
        chunk[0] = clamp_u16(r);

        // Green
        let g = chunk[1] as f32 * g_scale;
        chunk[1] = clamp_u16(g);

        // Blue
        let b = chunk[2] as f32 * b_scale;
        chunk[2] = clamp_u16(b);
    }
}

/// Apply a color matrix to an RGB image.
///
/// The matrix is a 3x3 row-major matrix.
/// [ R_out ]   [ m0 m1 m2 ] [ R_in ]
/// [ G_out ] = [ m3 m4 m5 ] [ G_in ]
/// [ B_out ]   [ m6 m7 m8 ] [ B_in ]
pub fn apply_color_matrix(image: &mut RgbImage, matrix: &[f32; 9]) {
    for chunk in image.data.chunks_exact_mut(3) {
        let r = chunk[0] as f32;
        let g = chunk[1] as f32;
        let b = chunk[2] as f32;

        let r_out = r * matrix[0] + g * matrix[1] + b * matrix[2];
        let g_out = r * matrix[3] + g * matrix[4] + b * matrix[5];
        let b_out = r * matrix[6] + g * matrix[7] + b * matrix[8];

        chunk[0] = clamp_u16(r_out);
        chunk[1] = clamp_u16(g_out);
        chunk[2] = clamp_u16(b_out);
    }
}

/// Apply gamma correction to an RGB image.
///
/// V_out = V_in ^ (1 / gamma)
/// Input and output are assumed to be in [0, 65535].
pub fn apply_gamma(image: &mut RgbImage, gamma: f32) {
    if (gamma - 1.0).abs() < 0.001 {
        return;
    }

    let inv_gamma = 1.0 / gamma;
    // Pre-compute lookup table for performance?
    // For 16-bit, a 65536 entry table is 128KB, reasonable.
    // Or just do per-pixel for now.

    // Optimization: Build LUT
    let mut lut = vec![0u16; 65536];
    for (i, v) in lut.iter_mut().enumerate() {
        let normalized = i as f32 / 65535.0;
        let corrected = normalized.powf(inv_gamma);
        *v = clamp_u16(corrected * 65535.0);
    }

    for pixel in &mut image.data {
        *pixel = lut[*pixel as usize];
    }
}

#[inline(always)]
fn clamp_u16(val: f32) -> u16 {
    if val < 0.0 {
        0
    } else if val > 65535.0 {
        65535
    } else {
        val as u16
    }
}
