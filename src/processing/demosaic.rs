use crate::core::image::{CfaPattern, RawImage, RgbImage};

/// Trait for demosaicing algorithms.
pub trait DemosaicMethod {
    /// Demosaic a raw image into an RGB image.
    fn demosaic(&self, raw: &RawImage) -> RgbImage;
}

/// Bilinear interpolation demosaicing algorithm.
pub struct Bilinear;

impl DemosaicMethod for Bilinear {
    fn demosaic(&self, raw: &RawImage) -> RgbImage {
        let width = raw.active_area.size.width;
        let height = raw.active_area.size.height;
        let x_offset = raw.active_area.origin.x;
        let y_offset = raw.active_area.origin.y;

        let mut data = Vec::with_capacity((width * height * 3) as usize);

        // Pre-calculate strides and bounds to avoid repeated checks in the loop
        let raw_width = raw.size.width;
        let raw_height = raw.size.height;

        let get_raw = |x: u32, y: u32| -> u16 {
            if x < raw_width && y < raw_height {
                raw.data[(y as usize) * (raw_width as usize) + (x as usize)]
            } else {
                0
            }
        };

        for y in 0..height {
            for x in 0..width {
                // Absolute coordinates in the raw image
                let abs_x = x + x_offset;
                let abs_y = y + y_offset;

                let (r, g, b) = demosaic_pixel_bilinear(abs_x, abs_y, raw.cfa_pattern, &get_raw);
                data.push(r);
                data.push(g);
                data.push(b);
            }
        }

        RgbImage {
            width,
            height,
            data,
        }
    }
}

/// Interpolates a single pixel using bilinear interpolation.
fn demosaic_pixel_bilinear<F>(x: u32, y: u32, pattern: CfaPattern, get_raw: &F) -> (u16, u16, u16)
where
    F: Fn(u32, u32) -> u16,
{
    // Determine the color of the current pixel location
    let (is_red, is_blue) = match pattern {
        CfaPattern::Rggb => ((x % 2 == 0) && (y % 2 == 0), (x % 2 == 1) && (y % 2 == 1)),
        CfaPattern::Grbg => ((x % 2 == 1) && (y % 2 == 0), (x % 2 == 0) && (y % 2 == 1)),
        CfaPattern::Gbrg => ((x % 2 == 0) && (y % 2 == 1), (x % 2 == 1) && (y % 2 == 0)),
        CfaPattern::Bggr => ((x % 2 == 1) && (y % 2 == 1), (x % 2 == 0) && (y % 2 == 0)),
    };

    // Green is true if it's neither red nor blue
    let is_green = !is_red && !is_blue;

    if is_green {
        // Current pixel is Green.
        // We need to interpolate Red and Blue.
        // One of them will be vertical neighbors, the other horizontal neighbors.

        let g = get_raw(x, y);

        let (r, b) = match pattern {
            // For RGGB:
            // R G R G
            // G B G B
            // If we are at (0,1) [Green], Left/Right is B, Up/Down is R
            // If we are at (1,0) [Green], Left/Right is R, Up/Down is B
            CfaPattern::Rggb | CfaPattern::Bggr => {
                if y % 2 == 0 {
                    // Even row
                    // R G R G (RGGB case) -> (x%2==1) -> Horizontal is R, Vertical is B
                    // B G B G (BGGR case) -> (x%2==1) -> Horizontal is B, Vertical is R
                    // checking pattern specifically:
                    if matches!(pattern, CfaPattern::Rggb) {
                        // Row 0: R G R G. We are at G. Neighbors (left/right) are R. Vertical are B.
                        (
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                        )
                    } else {
                        // BGGR
                        // Row 0: B G B G. We are at G. Neighbors (left/right) are B. Vertical are R.
                        (
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                        )
                    }
                } else {
                    // Odd row
                    // G B G B (RGGB case). We are at G. Left/Right B, Vert R.
                    // G R G R (BGGR case). We are at G. Left/Right R, Vert B.
                    if matches!(pattern, CfaPattern::Rggb) {
                        (
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                        )
                    } else {
                        (
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                        )
                    }
                }
            }
            CfaPattern::Grbg | CfaPattern::Gbrg => {
                // Similar logic...
                // GRBG:
                // G R G R
                // B G B G
                if y % 2 == 0 {
                    // Even row G R G R. At G. Horizontal R, Vert B.
                    if matches!(pattern, CfaPattern::Grbg) {
                        (
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                        )
                    } else {
                        // GBRG: G B G B. At G. Horizontal B, Vert R.
                        (
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                        )
                    }
                } else {
                    // Odd row
                    // GRBG: B G B G. At G. Horizontal B, Vert R.
                    if matches!(pattern, CfaPattern::Grbg) {
                        (
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                        )
                    } else {
                        // GBRG: R G R G. At G. Horizontal R, Vert B.
                        (
                            avg2(get_raw(x.saturating_sub(1), y), get_raw(x + 1, y)),
                            avg2(get_raw(x, y.saturating_sub(1)), get_raw(x, y + 1)),
                        )
                    }
                }
            }
        };
        (r, g, b)
    } else if is_red {
        // Current is Red.
        let r = get_raw(x, y);
        // Green is average of 4 cross neighbors (up, down, left, right)
        let g = avg4(
            get_raw(x, y.saturating_sub(1)),
            get_raw(x, y + 1),
            get_raw(x.saturating_sub(1), y),
            get_raw(x + 1, y),
        );
        // Blue is average of 4 diagonal neighbors
        let b = avg4(
            get_raw(x.saturating_sub(1), y.saturating_sub(1)),
            get_raw(x + 1, y.saturating_sub(1)),
            get_raw(x.saturating_sub(1), y + 1),
            get_raw(x + 1, y + 1),
        );
        (r, g, b)
    } else {
        // Current is Blue.
        let b = get_raw(x, y);
        // Green is average of 4 cross neighbors
        let g = avg4(
            get_raw(x, y.saturating_sub(1)),
            get_raw(x, y + 1),
            get_raw(x.saturating_sub(1), y),
            get_raw(x + 1, y),
        );
        // Red is average of 4 diagonal neighbors
        let r = avg4(
            get_raw(x.saturating_sub(1), y.saturating_sub(1)),
            get_raw(x + 1, y.saturating_sub(1)),
            get_raw(x.saturating_sub(1), y + 1),
            get_raw(x + 1, y + 1),
        );
        (r, g, b)
    }
}

#[inline(always)]
fn avg2(a: u16, b: u16) -> u16 {
    ((a as u32 + b as u32) / 2) as u16
}

#[inline(always)]
fn avg4(a: u16, b: u16, c: u16, d: u16) -> u16 {
    ((a as u32 + b as u32 + c as u32 + d as u32) / 4) as u16
}

// TODO: AHD, VHG algorithms
