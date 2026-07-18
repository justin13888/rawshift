//! Device-gated VAAPI integration tests: real hardware decode.
//!
//! Every test skips gracefully (eprintln + return) when the machine has no
//! `/dev/dri/renderD*` node, libva cannot be dlopen'd, the driver lacks the
//! codec, or the ffmpeg fixture generator is unavailable — CI without a GPU
//! stays green. On a machine with a working VAAPI driver (Intel/AMD, or
//! NVIDIA via nvidia-vaapi-driver) they exercise the full decode path:
//! bitstream generation → hvcC/av1C + payload → `decoder()` →
//! `DecodedFrame` pixels.
//!
//! Only compiled when a VAAPI build is requested; the test bodies are
//! `cfg`-gated on the build-script-selected backend so a plain
//! `cargo test -p rawshift-hwdec` (stub build) compiles them out.

#![cfg(hwdec_backend = "vaapi")]

use rawshift_hwdec::{
    ChromaSubsampling, CodecConfig, DecodedFrame, HwBackend, HwCodec, PixelFormat,
    StillDecodeRequest, available_codecs, backend, decoder,
};
use std::process::Command;

// ── gating helpers ──────────────────────────────────────────────────────────

/// Whether any DRM render node exists (cheap pre-check for skip messages).
fn has_render_node() -> bool {
    (128..136).any(|minor| std::path::Path::new(&format!("/dev/dri/renderD{minor}")).exists())
}

macro_rules! device_or_skip {
    ($codec:expr) => {
        match decoder($codec) {
            Some(decoder) => decoder,
            None => {
                eprintln!(
                    "Skipping VAAPI device test: no usable {} decoder \
                     (render node present: {})",
                    $codec,
                    has_render_node()
                );
                return;
            }
        }
    };
}

/// Run ffmpeg, returning false (→ skip) when it is missing or fails.
fn ffmpeg(args: &[&str]) -> bool {
    match Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(args)
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Per-plane sample variance must be non-zero for a real test pattern —
/// catches "decode succeeded but wrote a blank surface".
fn luma_variance(frame: &DecodedFrame) -> f64 {
    let plane = &frame.planes()[0];
    let bps = frame.format().bytes_per_sample();
    let width = frame.width() as usize;
    let mut samples: Vec<f64> = Vec::new();
    for row in 0..frame.height() as usize {
        let start = row * plane.stride;
        for px in 0..width {
            let value = if bps == 1 {
                f64::from(plane.data[start + px])
            } else {
                f64::from(u16::from_le_bytes([
                    plane.data[start + px * 2],
                    plane.data[start + px * 2 + 1],
                ]))
            };
            samples.push(value);
        }
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    samples.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / samples.len() as f64
}

// ── Annex-B → hvcC + length-prefixed payload (test-side container glue) ─────

/// Split an Annex-B stream into NAL units (3- or 4-byte start codes).
fn split_annex_b(data: &[u8]) -> Vec<Vec<u8>> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            starts.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut nals = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(data.len());
        let mut nal = &data[start + 3..end];
        // Trim the trailing zero(s) that belong to the next 4-byte start
        // code / trailing_zero_8bits.
        while let Some((&0, rest)) = nal.split_last() {
            nal = rest;
        }
        if !nal.is_empty() {
            nals.push(nal.to_vec());
        }
    }
    nals
}

/// Build an hvcC record (4-byte length prefixes) carrying `param_sets` and
/// a length-prefixed payload from `slices`.
fn build_hvcc_and_payload(nals: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
    let mut hvcc = vec![0u8; 23];
    hvcc[0] = 1; // configurationVersion
    hvcc[21] = 0x03; // lengthSizeMinusOne = 3
    let mut arrays: Vec<(u8, Vec<&[u8]>)> = Vec::new();
    let mut payload = Vec::new();
    for nal in nals {
        let nal_type = (nal[0] >> 1) & 0x3f;
        match nal_type {
            32..=34 => match arrays.iter_mut().find(|(t, _)| *t == nal_type) {
                Some((_, list)) => list.push(nal),
                None => arrays.push((nal_type, vec![nal])),
            },
            _ if nal_type < 32 => {
                payload.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                payload.extend_from_slice(nal);
            }
            _ => {} // SEI etc.
        }
    }
    hvcc[22] = arrays.len() as u8;
    for (nal_type, list) in &arrays {
        hvcc.push(0x80 | nal_type);
        hvcc.extend_from_slice(&(list.len() as u16).to_be_bytes());
        for nal in list {
            hvcc.extend_from_slice(&(nal.len() as u16).to_be_bytes());
            hvcc.extend_from_slice(nal);
        }
    }
    (hvcc, payload)
}

/// Generate one intra HEVC frame with ffmpeg/libx265 and decode it through
/// the hardware; returns `None` when the encoder is unavailable (→ skip).
fn decode_generated_hevc(pix_fmt: &str, tag: &str) -> Option<DecodedFrame> {
    let path = std::env::temp_dir().join(format!("rawshift_vaapi_test_{tag}.265"));
    let path_str = path.to_str().unwrap();
    if !ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=64x64:duration=1:rate=1",
        "-pix_fmt",
        pix_fmt,
        "-c:v",
        "libx265",
        "-x265-params",
        "keyint=1:log-level=none",
        "-frames:v",
        "1",
        "-f",
        "hevc",
        path_str,
    ]) {
        eprintln!("Skipping VAAPI HEVC decode test: ffmpeg/libx265 unavailable");
        return None;
    }
    let annex_b = std::fs::read(&path).expect("read generated bitstream");
    let _ = std::fs::remove_file(&path);
    let nals = split_annex_b(&annex_b);
    assert!(!nals.is_empty(), "generated stream has NAL units");
    let (hvcc, payload) = build_hvcc_and_payload(&nals);
    assert!(!payload.is_empty(), "generated stream has a coded slice");

    let mut decoder = device_or_skip_opt(HwCodec::Hevc)?;
    let request = StillDecodeRequest {
        config: CodecConfig::Hvcc(&hvcc),
        payload: &payload,
        width: 64,
        height: 64,
        bit_depth: if pix_fmt.contains("10") { 10 } else { 8 },
        chroma: ChromaSubsampling::Cs420,
    };
    Some(
        decoder
            .decode_still(&request)
            .expect("hardware HEVC decode"),
    )
}

/// `device_or_skip!` as a function usable from helpers.
fn device_or_skip_opt(codec: HwCodec) -> Option<Box<dyn rawshift_hwdec::HwStillDecoder>> {
    let decoder = decoder(codec);
    if decoder.is_none() {
        eprintln!(
            "Skipping VAAPI device test: no usable {codec} decoder \
             (render node present: {})",
            has_render_node()
        );
    }
    decoder
}

// ── (a) library + device open probe ─────────────────────────────────────────

/// On a machine with a driver: the probe must report VAAPI and consistent
/// codec/decoder availability. Without one: everything reports "none".
#[test]
fn vaapi_probe_reports_consistent_availability() {
    let codecs = available_codecs();
    match backend() {
        Some(backend_kind) => {
            assert_eq!(backend_kind, HwBackend::Vaapi);
            assert!(
                !codecs.is_empty(),
                "a reported backend must decode something"
            );
            eprintln!("VAAPI probe: available codecs: {codecs:?}");
        }
        None => {
            assert!(codecs.is_empty());
            eprintln!(
                "Skipping VAAPI probe assertions: no usable driver \
                 (render node present: {})",
                has_render_node()
            );
            return;
        }
    }
    for &codec in codecs {
        assert!(
            decoder(codec).is_some(),
            "decoder({codec}) must exist when listed"
        );
    }
}

// ── (b) HEVC Main real decode ───────────────────────────────────────────────

#[test]
fn vaapi_decodes_real_hevc_main_to_nv12() {
    let Some(frame) = decode_generated_hevc("yuv420p", "main8") else {
        return;
    };
    assert_eq!((frame.width(), frame.height()), (64, 64));
    assert_eq!(frame.format(), PixelFormat::Nv12);
    assert_eq!(frame.bit_depth(), 8);
    assert_eq!(frame.planes().len(), 2);
    let variance = luma_variance(&frame);
    assert!(
        variance > 10.0,
        "decoded test pattern must have real content (variance {variance})"
    );
    eprintln!("VAAPI HEVC Main: 64x64 NV12 decoded, luma variance {variance:.1}");
}

// ── (b') HEVC Main10 real decode ────────────────────────────────────────────

#[test]
fn vaapi_decodes_real_hevc_main10_to_p010() {
    let Some(frame) = decode_generated_hevc("yuv420p10le", "main10") else {
        return;
    };
    assert_eq!((frame.width(), frame.height()), (64, 64));
    assert_eq!(frame.format(), PixelFormat::P010);
    assert_eq!(frame.bit_depth(), 10);
    let variance = luma_variance(&frame);
    // P010 stores the value in the high bits of 16-bit words, so a real
    // pattern has large variance.
    assert!(
        variance > 100.0,
        "decoded 10-bit pattern must have real content (variance {variance})"
    );
    eprintln!("VAAPI HEVC Main10: 64x64 P010 decoded, luma variance {variance:.1}");
}

// ── (c) AV1 Profile 0 real decode ───────────────────────────────────────────

/// Minimal IVF demux: return the first frame's OBU stream.
fn first_ivf_frame(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 32 || &data[0..4] != b"DKIF" {
        return None;
    }
    let header_len = u16::from_le_bytes([data[6], data[7]]) as usize;
    let frame_size = u32::from_le_bytes(data[header_len..header_len + 4].try_into().ok()?) as usize;
    let start = header_len + 12;
    data.get(start..start + frame_size).map(<[u8]>::to_vec)
}

#[test]
fn vaapi_decodes_real_av1_profile0_to_nv12() {
    let path = std::env::temp_dir().join("rawshift_vaapi_test_av1.ivf");
    let path_str = path.to_str().unwrap();
    if !ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=64x64:duration=1:rate=1",
        "-pix_fmt",
        "yuv420p",
        "-c:v",
        "libaom-av1",
        "-still-picture",
        "1",
        "-crf",
        "40",
        "-b:v",
        "0",
        "-f",
        "ivf",
        path_str,
    ]) {
        eprintln!("Skipping VAAPI AV1 decode test: ffmpeg/libaom unavailable");
        return;
    }
    let ivf = std::fs::read(&path).expect("read generated IVF");
    let _ = std::fs::remove_file(&path);
    let payload = first_ivf_frame(&ivf).expect("IVF frame");

    // Minimal av1C: marker/version, profile 0 + level, flags byte, reserved;
    // the sequence header travels in the payload's OBU stream.
    let av1c = [0x81u8, 0x00, 0x0c, 0x00];

    let Some(mut decoder) = device_or_skip_opt(HwCodec::Av1) else {
        return;
    };
    let request = StillDecodeRequest {
        config: CodecConfig::Av1c(&av1c),
        payload: &payload,
        width: 64,
        height: 64,
        bit_depth: 8,
        chroma: ChromaSubsampling::Cs420,
    };
    let frame = decoder.decode_still(&request).expect("hardware AV1 decode");
    assert_eq!((frame.width(), frame.height()), (64, 64));
    assert_eq!(frame.format(), PixelFormat::Nv12);
    let variance = luma_variance(&frame);
    assert!(
        variance > 10.0,
        "decoded AV1 pattern must have real content (variance {variance})"
    );
    eprintln!("VAAPI AV1 Profile0: 64x64 NV12 decoded, luma variance {variance:.1}");
}

// ── error path on real hardware ─────────────────────────────────────────────

/// Garbage input must fail with a decode error, not a panic or a blank
/// frame.
#[test]
fn vaapi_rejects_garbage_bitstream() {
    let mut decoder = device_or_skip!(HwCodec::Hevc);
    let hvcc = {
        let mut v = vec![0u8; 23];
        v[0] = 1;
        v[21] = 0x03;
        v
    };
    let request = StillDecodeRequest {
        config: CodecConfig::Hvcc(&hvcc),
        payload: &[0, 0, 0, 4, 0x26, 0x01, 0xDE, 0xAD],
        width: 0,
        height: 0,
        bit_depth: 8,
        chroma: ChromaSubsampling::Cs420,
    };
    let err = decoder.decode_still(&request).unwrap_err();
    eprintln!("VAAPI correctly rejected garbage: {err}");
}
