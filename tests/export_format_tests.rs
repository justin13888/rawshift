//! Integration tests for image export with metadata embedding.
//!
//! Tests verify that EXIF and ICC metadata are correctly embedded in
//! exported images when the corresponding options are enabled.
//! These tests use actual RAW files via RawFile::export().

use rawshift::formats::export::{
    AvifOptions, EncodeOptions, HeicOptions, JpegOptions, JxlOptions, PngOptions, TiffOptions,
    WebPOptions,
};
use rawshift::formats::RawFile;
use rawshift::processing::ProcessingOptions;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

/// Path to test ARW file
fn test_arw_path() -> PathBuf {
    PathBuf::from("test_data/SONY/ILCE-6700/_JIC7790.ARW")
}

/// Get a temporary file path for test output
fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rawshift_test_{}", name));
    path
}

/// Check if JPEG data contains EXIF APP1 marker
fn jpeg_has_exif(data: &[u8]) -> bool {
    // EXIF is in APP1 marker: 0xFF 0xE1
    // Search for APP1 marker followed by "Exif\0\0"
    for i in 0..data.len().saturating_sub(10) {
        if data[i] == 0xFF && data[i + 1] == 0xE1 {
            // Check for "Exif" signature after the length bytes
            if i + 8 < data.len() && &data[i + 4..i + 8] == b"Exif" {
                return true;
            }
        }
    }
    false
}

/// Check if JPEG data contains ICC APP2 marker
fn jpeg_has_icc(data: &[u8]) -> bool {
    // ICC profile is in APP2 marker: 0xFF 0xE2
    // Search for APP2 marker followed by "ICC_PROFILE"
    for i in 0..data.len().saturating_sub(16) {
        if data[i] == 0xFF && data[i + 1] == 0xE2 {
            // Check for "ICC_PROFILE" signature
            if i + 15 < data.len() && &data[i + 4..i + 15] == b"ICC_PROFILE" {
                return true;
            }
        }
    }
    false
}

/// Check if WebP data contains EXIF chunk
fn webp_has_exif(data: &[u8]) -> bool {
    // WebP EXIF is in "EXIF" chunk within RIFF container
    for i in 0..data.len().saturating_sub(4) {
        if &data[i..i + 4] == b"EXIF" {
            return true;
        }
    }
    false
}

/// Open test ARW file
fn open_test_raw() -> RawFile<BufReader<File>> {
    let path = test_arw_path();
    let file = File::open(&path).expect("Open test ARW file");
    let reader = BufReader::new(file);
    RawFile::open(reader).expect("Parse test ARW file")
}

// ============================================================================
// JPEG Export Tests via RawFile::export()
// ============================================================================

mod jpeg_tests {
    use super::*;

    #[test]
    fn test_jpeg_export_with_exif_enabled() {
        let mut raw = open_test_raw();
        let path = temp_path("export_with_exif.jpg");

        let opts = JpegOptions {
            quality: 85,
            embed_exif: true,
            embed_icc: false,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts),
        )
        .expect("Export JPEG");

        // Verify file was created
        assert!(path.exists(), "JPEG file should exist");

        // Read and verify EXIF is present
        let data = fs::read(&path).expect("Read JPEG");
        assert!(data.len() > 100, "JPEG should not be empty");
        assert_eq!(&data[0..2], &[0xFF, 0xD8], "Should be valid JPEG");
        assert!(jpeg_has_exif(&data), "JPEG should contain EXIF metadata");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_jpeg_export_with_icc_enabled() {
        let mut raw = open_test_raw();
        let path = temp_path("export_with_icc.jpg");

        let opts = JpegOptions {
            quality: 85,
            embed_exif: false,
            embed_icc: true,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts),
        )
        .expect("Export JPEG");

        // Verify file was created
        assert!(path.exists(), "JPEG file should exist");

        // Read and verify ICC is present
        let data = fs::read(&path).expect("Read JPEG");
        assert!(jpeg_has_icc(&data), "JPEG should contain ICC profile");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_jpeg_export_with_both_exif_and_icc() {
        let mut raw = open_test_raw();
        let path = temp_path("export_with_both.jpg");

        let opts = JpegOptions {
            quality: 90,
            embed_exif: true,
            embed_icc: true,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts),
        )
        .expect("Export JPEG");

        // Read and verify both are present
        let data = fs::read(&path).expect("Read JPEG");
        assert!(jpeg_has_exif(&data), "JPEG should contain EXIF");
        assert!(jpeg_has_icc(&data), "JPEG should contain ICC");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_jpeg_export_without_metadata() {
        let mut raw = open_test_raw();
        let path = temp_path("export_no_meta.jpg");

        let opts = JpegOptions {
            quality: 85,
            embed_exif: false,
            embed_icc: false,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts),
        )
        .expect("Export JPEG");

        // Read and verify neither is present
        let data = fs::read(&path).expect("Read JPEG");
        assert!(!jpeg_has_exif(&data), "JPEG should NOT contain EXIF");
        assert!(!jpeg_has_icc(&data), "JPEG should NOT contain ICC");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_jpeg_export_default_options() {
        let mut raw = open_test_raw();
        let path = temp_path("export_default.jpg");

        // Default options should have embed_exif=true and embed_icc=true
        raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::jpeg())
            .expect("Export JPEG");

        let data = fs::read(&path).expect("Read JPEG");
        assert!(jpeg_has_exif(&data), "Default JPEG should contain EXIF");
        assert!(jpeg_has_icc(&data), "Default JPEG should contain ICC");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_jpeg_quality_affects_file_size() {
        let mut raw = open_test_raw();
        let path_low = temp_path("quality_low.jpg");
        let path_high = temp_path("quality_high.jpg");

        // Low quality
        let opts_low = JpegOptions {
            quality: 30,
            embed_exif: false,
            embed_icc: false,
        };
        raw.export(
            &path_low,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts_low),
        )
        .expect("Export low quality");

        // Need to re-open for second export
        let mut raw2 = open_test_raw();
        let opts_high = JpegOptions {
            quality: 95,
            embed_exif: false,
            embed_icc: false,
        };
        raw2.export(
            &path_high,
            &ProcessingOptions::default(),
            &EncodeOptions::Jpeg(opts_high),
        )
        .expect("Export high quality");

        let size_low = fs::metadata(&path_low).expect("Get size").len();
        let size_high = fs::metadata(&path_high).expect("Get size").len();

        // High quality should be significantly larger
        assert!(
            size_high > size_low,
            "High quality ({}) should be larger than low quality ({})",
            size_high,
            size_low
        );

        fs::remove_file(&path_low).ok();
        fs::remove_file(&path_high).ok();
    }
}

// ============================================================================
// WebP Export Tests via RawFile::export()
// ============================================================================

mod webp_tests {
    use super::*;

    #[test]
    fn test_webp_export_with_exif_enabled() {
        let mut raw = open_test_raw();
        let path = temp_path("export_with_exif.webp");

        let opts = WebPOptions {
            quality: 80.0,
            lossless: true,
            embed_exif: true,
            embed_icc: false,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::WebP(opts),
        )
        .expect("Export WebP");

        // Verify file was created
        assert!(path.exists(), "WebP file should exist");

        // Read and verify format and EXIF presence
        let data = fs::read(&path).expect("Read WebP");
        assert!(data.len() > 12, "WebP should not be empty");
        assert_eq!(&data[0..4], b"RIFF", "Should start with RIFF");
        assert_eq!(&data[8..12], b"WEBP", "Should have WEBP FourCC");
        assert!(webp_has_exif(&data), "WebP should contain EXIF metadata");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_webp_export_without_exif() {
        let mut raw = open_test_raw();
        let path = temp_path("export_no_exif.webp");

        let opts = WebPOptions {
            quality: 80.0,
            lossless: true,
            embed_exif: false,
            embed_icc: false,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::WebP(opts),
        )
        .expect("Export WebP");

        let data = fs::read(&path).expect("Read WebP");
        assert!(!webp_has_exif(&data), "WebP should NOT contain EXIF");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_webp_export_default_options() {
        let mut raw = open_test_raw();
        let path = temp_path("export_default.webp");

        // Default options should have embed_exif=true
        raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::webp())
            .expect("Export WebP");

        let data = fs::read(&path).expect("Read WebP");
        assert!(webp_has_exif(&data), "Default WebP should contain EXIF");

        fs::remove_file(&path).ok();
    }
}

// ============================================================================
// PNG Export Tests via RawFile::export()
// ============================================================================

mod png_tests {
    use super::*;

    #[test]
    fn test_png_export_basic() {
        let mut raw = open_test_raw();
        let path = temp_path("export_basic.png");

        raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::png())
            .expect("Export PNG");

        // Verify file
        let data = fs::read(&path).expect("Read PNG");
        assert!(data.len() > 8, "PNG should not be empty");
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(
            &data[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "Should have PNG signature"
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_png_export_8bit() {
        let mut raw = open_test_raw();
        let path = temp_path("export_8bit.png");

        let opts = PngOptions {
            bit_depth: zune_core::bit_depth::BitDepth::Eight,
        };

        raw.export(
            &path,
            &ProcessingOptions::default(),
            &EncodeOptions::Png(opts),
        )
        .expect("Export 8-bit PNG");

        assert!(path.exists());
        fs::remove_file(&path).ok();
    }
}

// ============================================================================
// TODO: Unimplemented Format Tests
// ============================================================================

// TODO: AVIF export tests
// Uncomment when AVIF encoding is implemented with ravif
mod avif_tests {
    use super::*;

    #[test]
    fn test_avif_options_default() {
        let opts = AvifOptions::default();
        assert_eq!(opts.quality, 80);
        assert_eq!(opts.speed, 6);
        assert!(opts.embed_exif);
    }

    // TODO: Uncomment when AVIF encoding is implemented
    // #[test]
    // fn test_avif_export_with_exif() {
    //     let mut raw = open_test_raw();
    //     let path = temp_path("export.avif");
    //     let opts = AvifOptions { quality: 80, speed: 6, embed_exif: true };
    //     raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::Avif(opts))
    //         .expect("Export AVIF");
    //     // Verify EXIF in AVIF container...
    //     fs::remove_file(&path).ok();
    // }
}

// TODO: HEIC export tests
#[cfg(feature = "heic")]
mod heic_tests {
    use super::*;

    #[test]
    fn test_heic_options_default() {
        let _opts = HeicOptions::default();
        // HEIC encoding not implemented
    }

    // TODO: Uncomment when HEIC encoding is implemented
    // #[test]
    // fn test_heic_export_basic() {
    //     let mut raw = open_test_raw();
    //     let path = temp_path("export.heic");
    //     raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::heic())
    //         .expect("Export HEIC");
    //     fs::remove_file(&path).ok();
    // }
}

// TODO: JXL export tests
mod jxl_tests {
    use super::*;

    #[test]
    fn test_jxl_options_default() {
        let _opts = JxlOptions::default();
        // JXL encoding not implemented
    }

    // TODO: Uncomment when JXL encoding is implemented
    // #[test]
    // fn test_jxl_export_with_exif() {
    //     let mut raw = open_test_raw();
    //     let path = temp_path("export.jxl");
    //     raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::jxl())
    //         .expect("Export JXL");
    //     // JXL supports EXIF in boxes
    //     fs::remove_file(&path).ok();
    // }
}

// TODO: TIFF export tests
mod tiff_tests {
    use super::*;

    #[test]
    fn test_tiff_options_default() {
        let _opts = TiffOptions::default();
        // TIFF encoding not implemented
    }

    // TODO: Uncomment when TIFF encoding is implemented
    // #[test]
    // fn test_tiff_export_basic() {
    //     let mut raw = open_test_raw();
    //     let path = temp_path("export.tiff");
    //     raw.export(&path, &ProcessingOptions::default(), &EncodeOptions::tiff())
    //         .expect("Export TIFF");
    //     fs::remove_file(&path).ok();
    // }
}

// ============================================================================
// EncodeOptions API Tests
// ============================================================================

mod encode_options_tests {
    use super::*;

    #[test]
    fn test_encode_options_constructors() {
        let _ = EncodeOptions::png();
        let _ = EncodeOptions::jpeg();
        let _ = EncodeOptions::avif();
        #[cfg(feature = "heic")]
        let _ = EncodeOptions::heic();
        let _ = EncodeOptions::jxl();
        let _ = EncodeOptions::webp();
        let _ = EncodeOptions::tiff();
        let _ = EncodeOptions::dng();
    }

    #[test]
    fn test_jpeg_options_defaults() {
        let opts = JpegOptions::default();
        assert_eq!(opts.quality, 90, "JPEG default quality should be 90");
        assert!(opts.embed_exif, "JPEG should embed EXIF by default");
        assert!(opts.embed_icc, "JPEG should embed ICC by default");
    }

    #[test]
    fn test_webp_options_defaults() {
        let opts = WebPOptions::default();
        assert_eq!(opts.quality, 80.0, "WebP default quality should be 80");
        assert!(opts.lossless, "WebP should be lossless by default");
        assert!(opts.embed_exif, "WebP should embed EXIF by default");
        assert!(opts.embed_icc, "WebP should embed ICC by default");
    }

    #[test]
    fn test_avif_options_defaults() {
        let opts = AvifOptions::default();
        assert_eq!(opts.quality, 80, "AVIF default quality should be 80");
        assert_eq!(opts.speed, 6, "AVIF default speed should be 6");
        assert!(opts.embed_exif, "AVIF should embed EXIF by default");
    }
}
