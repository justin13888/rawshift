//! Parse all metadata from image

use rawshift::formats::arw::ArwFile;
use std::env;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("Usage: metadata_dump <file.ARW>");

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut arw = ArwFile::parse(reader)?;

    // Extract metadata
    let metadata = arw.metadata().expect("Failed to extract metadata");
    println!("Metadata: {:#?}", metadata);

    // Read compressed raw data
    let raw_bytes = arw.read_raw_data()?;
    println!("Raw data size: {} bytes", raw_bytes.len());

    Ok(())
}
