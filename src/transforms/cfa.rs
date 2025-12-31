//! Color Filter Array (CFA) Demosaicing.
//!
//! This transform module acts as a high-level wrapper around the demosaicing algorithms
//! defined in [`crate::processing::demosaic`]. It handles the selection of the appropriate
//! algorithm (Bilinear, AHD, etc.) and manages the conversion from raw sensor data to
//! RGB image buffers.

use crate::core::image::{RawImage, RgbImage};
use crate::error::RawResult;
use crate::processing::demosaic::{Bilinear, DemosaicMethod};

/// Available demosaicing algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemosaicAlgorithm {
    /// Fast, simple bilinear interpolation.
    /// Good for previews or speed-critical applications.
    Bilinear,
    /// Adaptive Homogeneity-Directed (AHD) interpolation.
    /// Better quality for fine details, but slower.
    Ahd,
}

/// Demosaicing transform pipeline step.
pub struct CfaTransform {
    algorithm: DemosaicAlgorithm,
}

impl CfaTransform {
    /// Create a new CFA transform with the specified algorithm.
    pub fn new(algorithm: DemosaicAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Apply demosaicing to a RawImage, producing an RgbImage.
    pub fn apply(&self, raw: &RawImage) -> RawResult<RgbImage> {
        match self.algorithm {
            DemosaicAlgorithm::Bilinear | DemosaicAlgorithm::Ahd => {
                // TODO: Implement AHD and switch here
                let method = Bilinear;
                // We use the `demosaic` convenience method which handles allocation
                Ok(method.demosaic(raw))
            }
        }
    }
}

// TODO: Flesh out this module
