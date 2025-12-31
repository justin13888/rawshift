//! DNG Opcode List processing.
//!
//! DNG Opcodes allow for complex, deferred image processing steps like:
//! - Lens distortion correction (Warping).
//! - Vignetting correction.
//! - Bad pixel fixing.
//! - Gain Maps (crucial for Apple ProRAW).
//!
//! Implementation priority:
//! 1. `FixBadPixelsConstant` / `FixBadPixelsList`
//! 2. `GainMap` (Opcode 0x2) - Required for proper ProRAW rendering.
//! 3. `WarpRectilinear` - Lens correction.

use crate::error::RawResult;

/// A single DNG Opcode.
pub enum DngOpcode {
    GainMap,
    WarpRectilinear,
    FixBadPixels,
    // ...
}

pub struct OpcodeProcessor {
    // TODO: Store list of parsed opcodes from `OpcodeList1`, `2`, `3`.
}

impl OpcodeProcessor {
    pub fn process(&self, _image_data: &mut [f32], _width: usize, _height: usize) -> RawResult<()> {
        // TODO: Iterate opcodes and apply them in order.
        // NOTE: OpcodeList1 is applied on Raw data (pre-demosaic).
        //       OpcodeList2 is applied on Linear Raw data (post-demosaic / linear DNG).
        //       OpcodeList3 is applied on RGB data (post-color).
        Ok(())
    }
}

// TODO: Flesh out this module
