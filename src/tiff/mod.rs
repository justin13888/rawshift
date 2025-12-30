//! Low-level TIFF engine.
//!
//! This module provides the core TIFF parsing functionality used by
//! all TIFF-based RAW formats (ARW, CR2, DNG, etc.).
//!
//! # Structure
//!
//! - [`parser`] - IFD parsing and navigation
//! - [`tags`] - Known TIFF tag definitions
//! - [`types`] - TIFF data types and values

mod parser;
mod tags;
mod types;

pub use parser::*;
pub use tags::*;
pub use types::*;
