//! Shared core types for the rawshift image/video processing libraries.
//!
//! This crate holds pure, stateless data structures with no decoding logic:
//! geometry ([`image::Size`], [`image::Point`], [`image::Rect`]), pixel sample
//! types ([`pixel`]), CFA patterns, the raw/RGB image containers, and the
//! format-agnostic [`metadata`] model.
//!
//! # Charter
//!
//! This crate exists to hold the vocabulary that **both** the image and video
//! libraries need, so neither has to depend on the other. That is its only
//! justification for existing: a type used by exactly one side belongs in that
//! side's crate, not here.
//!
//! Video is parked for v1 and `rawshift-video` currently depends on nothing, so
//! the split is forward-looking rather than load-bearing today. Recording it
//! now keeps the boundary from drifting into "everything shared-ish lives in
//! core". The division is:
//!
//! **Genuinely video-shared** — media-agnostic, and video will consume these
//! as-is:
//!
//! - Geometry — [`image::Point`], [`image::Rect`], and frame dimensions.
//! - Codec descriptors — [`codec::CodecId`], [`codec::CodecInfo`],
//!   [`codec::CodecDirection`]. A codec registry spans stills and video.
//! - Metadata model — [`metadata::ImageMetadata`] and its
//!   [`metadata::MetadataEntry`]/[`metadata::MetadataKey`]/[`metadata::MetadataValue`]/[`metadata::MetadataNamespace`]
//!   parts, plus [`codec::MetadataEmbedOptions`]. EXIF/XMP semantics are the
//!   same for a video file as for a still.
//! - Rationals — [`metadata::URational`], [`metadata::SRational`]. These are the
//!   EXIF/TIFF wire representation, shared with the metadata model above.
//! - Color and bit depth descriptors, which describe a decoded frame regardless
//!   of whether it came from a still or a video track.
//!
//! **Stills-only** — present here for historical reasons, and not part of the
//! shared vocabulary. These describe a Bayer/X-Trans sensor mosaic, a concept
//! with no video analogue:
//!
//! - [`image::RawImage`] and [`image::RawImageBuilder`].
//! - [`image::CfaPattern`], [`image::XTransPattern`],
//!   [`image::white_level_from_bit_depth`].
//! - The RGB image container and the [`pixel`] sample types.
#![forbid(unsafe_code)]

pub mod codec;
pub mod color;
pub mod image;
pub mod metadata;
pub mod pixel;

pub use codec::{CodecDirection, CodecId, CodecInfo, MetadataEmbedOptions};
pub use color::{BitDepth, ColorSpace};
pub use image::XTransPattern;
pub use metadata::{
    ImageMetadata, MetadataEntry, MetadataExtractor, MetadataKey, MetadataNamespace, MetadataValue,
};
pub use pixel::{FromF32, Rgb, Rgb8, Rgb16, RgbF32, Rgba, Rgba8, Rgba16, RgbaF32, Sample};
