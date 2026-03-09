//! Black level subtraction for raw sensor data.
//!
//! Subtracts the per-channel black level (pedestal) from raw CFA data.
//! Black levels are stored per 2×2 Bayer position in [`RawImage::black_levels`].

use crate::core::image::RawImage;

/// Subtract per-channel black levels from raw CFA data in place.
///
/// Each pixel's black level is determined by its position in the 2×2 Bayer
/// pattern: `black_levels[(y % 2) * 2 + (x % 2)]`.
///
/// This is a no-op if all black levels are zero.
pub fn apply_black_level(raw: &mut RawImage) {
    let bl = raw.black_levels;
    if bl[0] == 0 && bl[1] == 0 && bl[2] == 0 && bl[3] == 0 {
        return;
    }

    let width = raw.size.width as usize;
    for (i, pixel) in raw.data.iter_mut().enumerate() {
        let x = i % width;
        let y = i / width;
        let bl_idx = (y % 2) * 2 + (x % 2);
        *pixel = pixel.saturating_sub(bl[bl_idx]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::image::{CfaPattern, Rect, Size};

    fn make_raw(width: u32, height: u32, data: Vec<u16>, black_levels: [u16; 4]) -> RawImage {
        let size = Size::new(width, height);
        let active = Rect::from_coords(0, 0, width, height);
        let mut raw = RawImage::new(size, active, 14, CfaPattern::Rggb);
        raw.data = data;
        raw.black_levels = black_levels;
        raw
    }

    #[test]
    fn no_op_when_all_zero() {
        let original = vec![100, 200, 300, 400];
        let mut raw = make_raw(2, 2, original.clone(), [0, 0, 0, 0]);
        apply_black_level(&mut raw);
        assert_eq!(raw.data, original);
    }

    #[test]
    fn uniform_black_level() {
        let mut raw = make_raw(2, 2, vec![100, 200, 300, 400], [50, 50, 50, 50]);
        apply_black_level(&mut raw);
        assert_eq!(raw.data, vec![50, 150, 250, 350]);
    }

    #[test]
    fn per_channel_subtraction() {
        // 2×2 image, each pixel at a different Bayer position
        // bl_idx mapping: (0,0)=0, (1,0)=1, (0,1)=2, (1,1)=3
        let mut raw = make_raw(2, 2, vec![1000, 1000, 1000, 1000], [10, 20, 30, 40]);
        apply_black_level(&mut raw);
        assert_eq!(raw.data, vec![990, 980, 970, 960]);
    }

    #[test]
    fn saturating_subtraction_does_not_underflow() {
        let mut raw = make_raw(2, 1, vec![5, 10], [100, 100, 100, 100]);
        apply_black_level(&mut raw);
        assert_eq!(raw.data, vec![0, 0]);
    }
}
