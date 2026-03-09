//! Camera color calibration database.
//!
//! Static lookup tables for camera-specific color matrices, sourced from
//! LibRaw/dcraw (Sony models) and DNG file metadata (Apple models).
//!
//! Color matrices follow the DNG specification: they transform XYZ values
//! to camera-native color space under a given calibration illuminant.

/// EXIF LightSource codes for calibration illuminants.
pub mod light_source {
    /// Standard Illuminant A (~2856K, incandescent)
    pub const STANDARD_LIGHT_A: u16 = 17;
    /// D55 (~5503K)
    pub const D55: u16 = 20;
    /// D65 (~6504K, daylight)
    pub const D65: u16 = 21;
    /// D75 (~7504K)
    pub const D75: u16 = 22;
    /// D50 (~5003K, ICC PCS)
    pub const D50: u16 = 23;
}

/// Camera color calibration data.
///
/// Stores one or two color matrices following the DNG dual-illuminant model.
/// When two matrices are present, raw processors can interpolate between them
/// based on the scene's correlated color temperature.
#[derive(Debug, Clone)]
pub struct CameraCalibration {
    /// Camera model identifier (e.g., "ILCE-7RM5")
    pub model: &'static str,
    /// Color matrix for calibration illuminant 1 (3x3, row-major, XYZ -> camera native)
    pub color_matrix_1: Option<[f64; 9]>,
    /// EXIF LightSource code for illuminant 1
    pub illuminant_1: Option<u16>,
    /// Color matrix for calibration illuminant 2 (3x3, row-major, XYZ -> camera native)
    pub color_matrix_2: Option<[f64; 9]>,
    /// EXIF LightSource code for illuminant 2
    pub illuminant_2: Option<u16>,
}

/// Camera color matrix database.
///
/// Sony matrices are sourced from LibRaw/dcraw `adobe_coeff` table (D65 only).
/// Apple matrices are sourced from ProRAW DNG file metadata (dual-illuminant).
static CAMERA_DB: &[CameraCalibration] = &[
    // ── Sony ──────────────────────────────────────────────────────────
    // Source: LibRaw colordata.cpp adobe_coeff table (values / 10000)
    CameraCalibration {
        model: "ILCE-7RM5",
        color_matrix_1: None,
        illuminant_1: None,
        color_matrix_2: Some([
            0.8200, -0.2976, -0.0719, -0.4296, 1.2053, 0.2532, -0.0429, 0.1282, 0.5774,
        ]),
        illuminant_2: Some(light_source::D65),
    },
    CameraCalibration {
        model: "ILCE-7M4",
        color_matrix_1: None,
        illuminant_1: None,
        color_matrix_2: Some([
            0.7460, -0.2365, -0.0588, -0.5687, 1.3442, 0.2474, -0.0624, 0.1156, 0.6584,
        ]),
        illuminant_2: Some(light_source::D65),
    },
    CameraCalibration {
        model: "ILCE-6700",
        color_matrix_1: None,
        illuminant_1: None,
        color_matrix_2: Some([
            0.6972, -0.2408, -0.0600, -0.4330, 1.2101, 0.2515, -0.0388, 0.1277, 0.5847,
        ]),
        illuminant_2: Some(light_source::D65),
    },
    // ── Apple ─────────────────────────────────────────────────────────
    // Source: ProRAW DNG embedded metadata
    CameraCalibration {
        model: "iPhone 13 Pro Max",
        color_matrix_1: Some([
            1.2270, -0.5450, -0.2610, -0.4550, 1.5180, -0.0430, -0.0410, 0.1640, 0.5910,
        ]),
        illuminant_1: Some(light_source::STANDARD_LIGHT_A),
        color_matrix_2: Some([
            0.9150, -0.3220, -0.1260, -0.4290, 1.3100, 0.0950, -0.1060, 0.2350, 0.4310,
        ]),
        illuminant_2: Some(light_source::D65),
    },
    CameraCalibration {
        model: "iPhone 16 Pro Max",
        color_matrix_1: Some([
            1.3092, -0.6653, -0.2359, -0.4257, 1.4791, -0.0241, -0.0360, 0.1377, 0.6341,
        ]),
        illuminant_1: Some(light_source::STANDARD_LIGHT_A),
        color_matrix_2: Some([
            0.9564, -0.3793, -0.1339, -0.4043, 1.2963, 0.0853, -0.0940, 0.2064, 0.4659,
        ]),
        illuminant_2: Some(light_source::D65),
    },
];

/// Look up camera calibration data by model string.
///
/// Performs an exact match against the model field.
pub fn get_camera_calibration(model: &str) -> Option<&'static CameraCalibration> {
    CAMERA_DB.iter().find(|c| c.model == model)
}

/// Look up camera calibration data by substring match.
///
/// Useful when the model string from EXIF contains extra text
/// (e.g., "Sony ILCE-7RM5" should match "ILCE-7RM5").
pub fn find_camera_calibration(model: &str) -> Option<&'static CameraCalibration> {
    CAMERA_DB
        .iter()
        .find(|c| model.contains(c.model) || c.model.contains(model))
}

/// Returns all camera calibrations in the database.
pub fn all_cameras() -> &'static [CameraCalibration] {
    CAMERA_DB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_lookup() {
        let cal = get_camera_calibration("ILCE-7RM5").unwrap();
        assert_eq!(cal.model, "ILCE-7RM5");
        assert!(cal.color_matrix_2.is_some());
        assert_eq!(cal.illuminant_2, Some(light_source::D65));
    }

    #[test]
    fn test_substring_lookup() {
        let cal = find_camera_calibration("Sony ILCE-7M4").unwrap();
        assert_eq!(cal.model, "ILCE-7M4");
    }

    #[test]
    fn test_iphone_dual_illuminant() {
        let cal = get_camera_calibration("iPhone 16 Pro Max").unwrap();
        assert!(cal.color_matrix_1.is_some());
        assert!(cal.color_matrix_2.is_some());
        assert_eq!(cal.illuminant_1, Some(light_source::STANDARD_LIGHT_A));
        assert_eq!(cal.illuminant_2, Some(light_source::D65));
    }

    #[test]
    fn test_unknown_camera() {
        assert!(get_camera_calibration("Unknown Camera").is_none());
    }

    #[test]
    fn test_all_cameras() {
        let cameras = all_cameras();
        assert_eq!(cameras.len(), 5);
    }

    #[test]
    fn test_matrix_values_valid() {
        for cam in all_cameras() {
            if let Some(m) = &cam.color_matrix_2 {
                // Row sums of a valid XYZ->camera matrix should be reasonable
                // (not all zeros, not wildly large)
                let row0_sum: f64 = m[0..3].iter().sum();
                let row1_sum: f64 = m[3..6].iter().sum();
                let row2_sum: f64 = m[6..9].iter().sum();
                assert!(
                    row0_sum.abs() < 3.0,
                    "row 0 sum out of range for {}",
                    cam.model
                );
                assert!(
                    row1_sum.abs() < 3.0,
                    "row 1 sum out of range for {}",
                    cam.model
                );
                assert!(
                    row2_sum.abs() < 3.0,
                    "row 2 sum out of range for {}",
                    cam.model
                );
            }
        }
    }
}
