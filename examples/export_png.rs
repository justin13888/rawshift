//! Export an image as PNG
//!
//! Usage: export_png <file.dng>
use std::fs::File;
use std::{env, io::BufReader};

use rawshift::prelude::{ProcessingOptions, RawFile};
use std::path::PathBuf;
use tracing_subscriber::{prelude::*, EnvFilter};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let path = env::args().nth(1).expect("Usage: export_png <file>");
    let path = PathBuf::from(&path);
    let output_path = PathBuf::from("output.png");

    use rawshift::formats::export::EncodeOptions;

    // ...
    let file = File::open(&path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut raw = RawFile::open(reader).expect("Failed to open image");
    println!("Opened image");
    raw.export(
        &output_path,
        &ProcessingOptions::default(),
        &EncodeOptions::default(),
    )
    .expect("Failed to export image");
    println!("Exported image at {}", output_path.display());
}
