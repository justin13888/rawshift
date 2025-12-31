use crate::core::image::{RawImage, RgbImage};

/// Error type for demosaicing operations.
#[derive(Debug, Clone)]
pub enum DemosaicError {
    /// Output buffer size does not match expected size
    BufferSizeMismatch {
        /// Expected buffer size in u16 elements
        expected: usize,
        /// Actual buffer size provided
        actual: usize,
    },
    /// Invalid image dimensions
    InvalidDimensions,
}

impl std::fmt::Display for DemosaicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DemosaicError::BufferSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "buffer size mismatch: expected {} elements, got {}",
                    expected, actual
                )
            }
            DemosaicError::InvalidDimensions => write!(f, "invalid image dimensions"),
        }
    }
}

impl std::error::Error for DemosaicError {}

/// Trait for demosaicing algorithms.
///
/// Implementors should override [`demosaic_into`](Self::demosaic_into) as the primary method.
/// The [`demosaic`](Self::demosaic) method provides a convenience wrapper that allocates output.
///
/// # Example
///
/// ```ignore
/// use rawshift::processing::demosaic::{Bilinear, DemosaicMethod};
///
/// let demosaiced = Bilinear.demosaic(&raw_image);
/// ```
pub trait DemosaicMethod {
    /// Demosaic a raw image into a pre-allocated RGB buffer.
    ///
    /// This is the primary method that implementors must override.
    /// The output buffer must have exactly `width * height * 3` elements.
    ///
    /// # Arguments
    /// * `raw` - The raw image to demosaic
    /// * `output` - Pre-allocated buffer for RGB output (interleaved R, G, B, R, G, B, ...)
    ///
    /// # Errors
    /// Returns [`DemosaicError::BufferSizeMismatch`] if buffer size is incorrect.
    fn demosaic_into(&self, raw: &RawImage, output: &mut [u16]) -> Result<(), DemosaicError>;

    /// Demosaic a raw image into a newly allocated RGB image.
    ///
    /// This is a convenience wrapper that allocates the output buffer
    /// and calls [`demosaic_into`](Self::demosaic_into).
    #[must_use]
    fn demosaic(&self, raw: &RawImage) -> RgbImage {
        let width = raw.active_area.size.width;
        let height = raw.active_area.size.height;
        let mut data = vec![0u16; (width as usize) * (height as usize) * 3];
        self.demosaic_into(raw, &mut data)
            .expect("demosaic_into failed with correctly sized buffer");
        RgbImage {
            width,
            height,
            data,
        }
    }
}

mod bilinear;
pub use bilinear::Bilinear;

// TODO: AHD, VHG algorithms

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demosaic_error_display() {
        let err = DemosaicError::BufferSizeMismatch {
            expected: 300,
            actual: 100,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("300"));
        assert!(msg.contains("100"));

        let err = DemosaicError::InvalidDimensions;
        let msg = format!("{}", err);
        assert!(msg.contains("dimension"));
    }
}
