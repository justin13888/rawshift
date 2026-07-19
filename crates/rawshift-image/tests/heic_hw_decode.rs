//! End-to-end HEIC hardware decode: `HeicFile::decode_primary` through
//! gamut-heic's pipeline into the rawshift-hwdec VAAPI backend (#29), on a
//! real GPU.
//!
//! Gated twice: compiled only with the `hw` feature, and each test skips
//! gracefully (eprintln + return) when no hardware HEVC decoder is usable at
//! runtime or the local fixture is missing — CI without a GPU stays green,
//! while a machine with a working VAAPI/VideoToolbox/MediaCodec driver
//! asserts real pixels.
//!
//! Fixture: `test_data/standard/heic/test_64x64.heic` (locally generated,
//! gitignored). Regenerate with libheif + ffmpeg:
//!
//! ```sh
//! ffmpeg -f lavfi -i testsrc2=size=64x64:duration=1:rate=1 -frames:v 1 /tmp/test64.png
//! heif-enc --thumb 32 -o test_data/standard/heic/test_64x64.heic /tmp/test64.png
//! ```

#![cfg(feature = "hw")]

use rawshift_image::formats::{HeicAuxKind, HeicFile, heic_hw_decode_available};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test_data/standard/heic/test_64x64.heic")
}

macro_rules! hw_fixture_or_skip {
    () => {{
        let path = fixture_path();
        let Ok(data) = std::fs::read(&path) else {
            eprintln!(
                "Skipping HEIC hardware decode test: fixture missing at {} \
                 (see module docs for the heif-enc regeneration command)",
                path.display()
            );
            return;
        };
        if !heic_hw_decode_available() {
            eprintln!(
                "Skipping HEIC hardware decode test: no hardware HEVC decoder \
                 usable at runtime on this machine"
            );
            return;
        }
        data
    }};
}

/// The full stack on real hardware: container parse (gamut-heic) → hvcC +
/// payload → rawshift-hwdec VAAPI decode → YCbCr→RGB presentation. Asserts
/// dimensions and plausible content (non-zero variance), not just success.
#[test]
fn heic_primary_hw_decodes_end_to_end_with_real_pixels() {
    let data = hw_fixture_or_skip!();
    let file = HeicFile::open(data).expect("open HEIC fixture");
    let image = file.decode_primary().expect("hardware decode primary");

    assert_eq!(
        (image.width(), image.height()),
        (64, 64),
        "fixture is a 64x64 still"
    );
    let samples = image.data();
    assert_eq!(samples.len(), 64 * 64 * 3);

    // Plausible content: the test image is a colour pattern, so the decoded
    // RGB must have real variance — a blank/garbage surface fails here.
    let mean = samples.iter().map(|&s| f64::from(s)).sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|&s| (f64::from(s) - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    assert!(
        variance > 1000.0,
        "decoded pixels must carry real content (variance {variance:.1}, mean {mean:.1})"
    );
    eprintln!(
        "HEIC end-to-end hardware decode: 64x64 RGB16, mean {mean:.1}, variance {variance:.1}"
    );
}

/// The advertised availability flag and the thumbnail path agree with real
/// hardware behaviour.
#[test]
fn heic_thumbnail_hw_decodes_when_listed() {
    let data = hw_fixture_or_skip!();
    let file = HeicFile::open(data).expect("open HEIC fixture");
    let has_thumb = file
        .aux_images()
        .iter()
        .any(|aux| aux.kind == HeicAuxKind::Thumbnail);
    let thumb = file.thumbnail().expect("thumbnail decode must not error");
    assert_eq!(
        has_thumb,
        thumb.is_some(),
        "thumbnail() must agree with aux listing under a real decoder"
    );
    if let Some(thumb) = thumb {
        assert!(thumb.width() > 0 && thumb.height() > 0);
        eprintln!(
            "HEIC thumbnail hardware decode: {}x{}",
            thumb.width(),
            thumb.height()
        );
    }
}
