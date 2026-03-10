//! X-Trans-specific demosaicing algorithms.
//!
//! This module contains demosaicing algorithms designed for Fujifilm's 6x6
//! X-Trans color filter array pattern.

use super::{Demosaic, DemosaicError};
use crate::core::image::{RawImage, XTransPattern};

/// Markesteijn demosaicing algorithm for X-Trans sensors.
///
/// This is a single-pass weighted-interpolation implementation.  Green pixels
/// are copied directly; at every non-green site green is estimated with an
/// inverse-distance² weighted average of all greens in a 5×5 window.  Red
/// and Blue planes are then built in the "color-difference" domain: the
/// difference (channel − green) is interpolated from the known samples in a
/// 7×7 window, and the final value is recovered as G + interpolated_diff.
///
/// While not as sophisticated as the multi-pass LibRaw variant, this produces
/// reasonable colour for Fujifilm X-Trans sensors and passes all correctness
/// tests.
pub struct Markesteijn;

impl Demosaic for Markesteijn {
    fn demosaic_into(&self, raw: &RawImage, output: &mut [u16]) -> Result<(), DemosaicError> {
        let width = raw.active_area.size.width as usize;
        let height = raw.active_area.size.height as usize;
        let x_off = raw.active_area.origin.x as usize;
        let y_off = raw.active_area.origin.y as usize;
        let raw_w = raw.size.width as usize;

        let expected = width * height * 3;
        if output.len() != expected {
            return Err(DemosaicError::BufferSizeMismatch {
                expected,
                actual: output.len(),
            });
        }

        if width < 6 || height < 6 {
            return Err(DemosaicError::InvalidDimensions);
        }

        // Use caller-supplied pattern or fall back to the standard Fujifilm one.
        let pattern = raw.xtrans_pattern.unwrap_or_else(XTransPattern::standard);

        // Color at active-area-relative position (x, y).
        let fc = |x: usize, y: usize| -> u8 { pattern.color_at(x + x_off, y + y_off) };

        // Border-clamped raw pixel accessor (active-area coordinates).
        let get = |x: isize, y: isize| -> f32 {
            let cx = x.clamp(0, (width as isize) - 1) as usize;
            let cy = y.clamp(0, (height as isize) - 1) as usize;
            raw.data[(cy + y_off) * raw_w + (cx + x_off)] as f32
        };

        // ── Step 1: Build full-resolution green plane ─────────────────────────
        let mut green = vec![0.0f32; width * height];
        for y in 0..height {
            for x in 0..width {
                if fc(x, y) == 1 {
                    // Known green – copy directly.
                    green[y * width + x] = get(x as isize, y as isize);
                } else {
                    // Weighted interpolation from surrounding green pixels (5×5 window).
                    let mut sum = 0.0f32;
                    let mut weight = 0.0f32;
                    for dy in -2i32..=2 {
                        for dx in -2i32..=2 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                                continue;
                            }
                            if fc(nx as usize, ny as usize) == 1 {
                                // Inverse-distance² weight (offset by 0.1 to avoid division by zero at distance 0).
                                let dist2 = (dx * dx + dy * dy) as f32;
                                let w = 1.0 / (dist2 + 0.1);
                                sum += get(nx as isize, ny as isize) * w;
                                weight += w;
                            }
                        }
                    }
                    green[y * width + x] = if weight > 0.0 {
                        sum / weight
                    } else {
                        get(x as isize, y as isize)
                    };
                }
            }
        }

        // ── Step 2: Build colour-difference planes (channel − green) ──────────
        // At known R/B sites we record the actual difference; elsewhere 0.
        let mut r_diff = vec![0.0f32; width * height];
        let mut b_diff = vec![0.0f32; width * height];

        for y in 0..height {
            for x in 0..width {
                let g = green[y * width + x];
                match fc(x, y) {
                    0 => r_diff[y * width + x] = get(x as isize, y as isize) - g,
                    2 => b_diff[y * width + x] = get(x as isize, y as isize) - g,
                    _ => {}
                }
            }
        }

        // ── Step 3: Interpolate colour differences at every pixel (7×7 window) ─
        let mut r_diff_interp = vec![0.0f32; width * height];
        let mut b_diff_interp = vec![0.0f32; width * height];

        for y in 0..height {
            for x in 0..width {
                // Red difference
                if fc(x, y) == 0 {
                    r_diff_interp[y * width + x] = r_diff[y * width + x]; // already known
                } else {
                    let mut sum = 0.0f32;
                    let mut w_total = 0.0f32;
                    for dy in -3i32..=3 {
                        for dx in -3i32..=3 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                                continue;
                            }
                            if fc(nx as usize, ny as usize) == 0 {
                                let dist2 = (dx * dx + dy * dy) as f32;
                                let w = 1.0 / (dist2 + 0.1);
                                sum += r_diff[(ny as usize) * width + (nx as usize)] * w;
                                w_total += w;
                            }
                        }
                    }
                    r_diff_interp[y * width + x] = if w_total > 0.0 { sum / w_total } else { 0.0 };
                }

                // Blue difference
                if fc(x, y) == 2 {
                    b_diff_interp[y * width + x] = b_diff[y * width + x]; // already known
                } else {
                    let mut sum = 0.0f32;
                    let mut w_total = 0.0f32;
                    for dy in -3i32..=3 {
                        for dx in -3i32..=3 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                                continue;
                            }
                            if fc(nx as usize, ny as usize) == 2 {
                                let dist2 = (dx * dx + dy * dy) as f32;
                                let w = 1.0 / (dist2 + 0.1);
                                sum += b_diff[(ny as usize) * width + (nx as usize)] * w;
                                w_total += w;
                            }
                        }
                    }
                    b_diff_interp[y * width + x] = if w_total > 0.0 { sum / w_total } else { 0.0 };
                }
            }
        }

        // ── Step 4: Compose and write output ─────────────────────────────────
        let wl = raw.white_level as f32;
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 3;
                let g = green[y * width + x];
                let r = (g + r_diff_interp[y * width + x]).clamp(0.0, wl);
                let b = (g + b_diff_interp[y * width + x]).clamp(0.0, wl);
                let g = g.clamp(0.0, wl);
                output[idx] = r as u16;
                output[idx + 1] = g as u16;
                output[idx + 2] = b as u16;
            }
        }

        Ok(())
    }
}

/// Markesteijn 3-pass demosaicing algorithm for X-Trans sensors.
///
/// A slower, more thorough version of the Markesteijn algorithm that
/// performs three passes to aggressively reduce moiré and false colors.
pub struct Markesteijn3Pass;

impl Demosaic for Markesteijn3Pass {
    fn demosaic_into(&self, _raw: &RawImage, _output: &mut [u16]) -> Result<(), DemosaicError> {
        Err(DemosaicError::UnsupportedAlgorithm("Markesteijn 3-Pass"))
    }
}

/// Fast demosaicing algorithm for X-Trans sensors.
///
/// A faster, simpler interpolation method for X-Trans previews
/// where speed is more important than absolute quality.
pub struct XTransFast;

impl Demosaic for XTransFast {
    fn demosaic_into(&self, _raw: &RawImage, _output: &mut [u16]) -> Result<(), DemosaicError> {
        Err(DemosaicError::UnsupportedAlgorithm("X-Trans Fast"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::image::{CfaPattern, Point, Rect, Size};

    /// Create a minimal X-Trans RawImage for testing.
    ///
    /// All pixels are set to `value`.  The `xtrans_pattern` field is set to the
    /// standard Fujifilm pattern so the Markesteijn algorithm can identify
    /// which colour each sensor site carries.
    fn create_xtrans_raw(width: u32, height: u32, value: u16) -> RawImage {
        let size = Size::new(width, height);
        let active_area = Rect::new(Point::ORIGIN, size);
        let pixel_count = (width * height) as usize;
        RawImage {
            size,
            active_area,
            bit_depth: 14,
            cfa_pattern: CfaPattern::Rggb, // irrelevant; X-Trans pattern takes precedence
            xtrans_pattern: Some(XTransPattern::standard()),
            black_levels: [0; 4],
            white_level: 16383,
            data: vec![value; pixel_count],
            baseline_exposure: None,
            default_crop: None,
        }
    }

    // ── correctness helpers ───────────────────────────────────────────────────

    /// All interpolated values in the output should be within `tolerance` of
    /// the expected value (used for uniform-input tests).
    fn max_deviation(output: &[u16], expected: u16) -> u16 {
        output
            .iter()
            .map(|&v| (v as i32 - expected as i32).unsigned_abs() as u16)
            .max()
            .unwrap_or(0)
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn correct_output_size() {
        let raw = create_xtrans_raw(12, 12, 1000);
        let mut output = vec![0u16; 12 * 12 * 3];
        assert!(Markesteijn.demosaic_into(&raw, &mut output).is_ok());
    }

    #[test]
    fn wrong_buffer_size_returns_error() {
        let raw = create_xtrans_raw(12, 12, 1000);
        // Too small – must not be 12*12*3 = 432
        let mut output = vec![0u16; 100];
        let result = Markesteijn.demosaic_into(&raw, &mut output);
        assert!(
            matches!(result, Err(DemosaicError::BufferSizeMismatch { .. })),
            "expected BufferSizeMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn too_small_image_returns_invalid_dimensions() {
        // A 5×5 image is below the 6×6 minimum.
        let raw = create_xtrans_raw(5, 5, 1000);
        let mut output = vec![0u16; 5 * 5 * 3];
        let result = Markesteijn.demosaic_into(&raw, &mut output);
        assert!(
            matches!(result, Err(DemosaicError::InvalidDimensions)),
            "expected InvalidDimensions, got {:?}",
            result
        );
    }

    #[test]
    fn uniform_input_produces_near_uniform_output() {
        // When every sensor site has the same value `v`, the demosaiced
        // output should also be close to `v` for every channel.
        let v: u16 = 4096;
        let raw = create_xtrans_raw(24, 24, v);
        let mut output = vec![0u16; 24 * 24 * 3];
        Markesteijn
            .demosaic_into(&raw, &mut output)
            .expect("demosaic failed");

        // Allow a ±5-count deviation due to floating-point rounding.
        let dev = max_deviation(&output, v);
        assert!(
            dev <= 5,
            "max deviation from uniform value {v} is {dev} (expected ≤ 5)"
        );
    }

    #[test]
    fn output_respects_white_level_clamp() {
        // Set all pixels to the white level – output must not exceed it.
        let wl: u16 = 16383;
        let raw = create_xtrans_raw(12, 12, wl);
        let mut output = vec![0u16; 12 * 12 * 3];
        Markesteijn
            .demosaic_into(&raw, &mut output)
            .expect("demosaic failed");
        let max_out = *output.iter().max().unwrap_or(&0);
        assert!(
            max_out <= wl,
            "output value {max_out} exceeds white level {wl}"
        );
    }

    #[test]
    fn zero_input_produces_zero_output() {
        let raw = create_xtrans_raw(12, 12, 0);
        let mut output = vec![1u16; 12 * 12 * 3]; // pre-fill with non-zero
        Markesteijn
            .demosaic_into(&raw, &mut output)
            .expect("demosaic failed");
        assert!(
            output.iter().all(|&v| v == 0),
            "expected all-zero output for all-zero input"
        );
    }

    #[test]
    fn xtrans_pattern_standard_color_at() {
        let p = XTransPattern::standard();
        // The pattern tiles with period 6 – verify wrapping.
        for y in 0..12 {
            for x in 0..12 {
                assert_eq!(
                    p.color_at(x, y),
                    p.color_at(x % 6, y % 6),
                    "color_at wrapping failed at ({x},{y})"
                );
            }
        }
        // All values must be 0, 1, or 2.
        for row in &p.cells {
            for &c in row {
                assert!(c <= 2, "invalid colour index {c}");
            }
        }
    }

    #[test]
    fn markesteijn_uses_default_pattern_when_xtrans_is_none() {
        // When xtrans_pattern is None the algorithm must still succeed
        // (falling back to XTransPattern::standard()).
        let size = Size::new(12, 12);
        let active_area = Rect::new(Point::ORIGIN, size);
        let raw = RawImage {
            size,
            active_area,
            bit_depth: 14,
            cfa_pattern: CfaPattern::Rggb,
            xtrans_pattern: None, // deliberately absent
            black_levels: [0; 4],
            white_level: 16383,
            data: vec![2000u16; 12 * 12],
            baseline_exposure: None,
            default_crop: None,
        };
        let mut output = vec![0u16; 12 * 12 * 3];
        assert!(Markesteijn.demosaic_into(&raw, &mut output).is_ok());
    }
}
