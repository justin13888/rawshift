use rawshift::formats::arw::ArwFile;
use rawshift::processing::color::{apply_color_matrix, apply_gamma, apply_white_balance};
use rawshift::processing::demosaic::{Bilinear, DemosaicMethod};
use std::env;
use std::fs::File;
use std::io::BufReader;
use zune_core::bit_depth::BitDepth;
use zune_core::colorspace::ColorSpace;
use zune_core::options::EncoderOptions;
use zune_png::PngEncoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: decode_arw <input.ARW> <output.png>");
        return Ok(());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Opening {}", input_path);
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut arw = ArwFile::parse(reader)?;

    println!("Decoding raw image...");
    let mut raw_image = match arw.decode_raw() {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Decoding failed: {}", e);
            return Ok(());
        }
    };

    println!(
        "Resolution: {}x{}",
        raw_image.size.width, raw_image.size.height
    );
    println!("Bit Depth: {}", raw_image.bit_depth);

    if let Some(meta) = arw.metadata() {
        println!("Active Area: {:?}", meta.active_area);
    }

    // 1. Black Level Subtraction
    // Sony cameras typically have a black level of 512 for 14-bit data.
    // We should subtract this before processing.
    // For simplicity, we use the first black level value.
    let black_level = raw_image.black_levels[0];
    println!("Subtracting Black Level: {}", black_level);
    for pixel in &mut raw_image.data {
        *pixel = pixel.saturating_sub(black_level);
    }

    // 2. Scale to 16-bit
    // Processing functions assume input is comparable to 16-bit range or normalized.
    // It is best to normalize to full 16-bit range early.
    let shift = 16u8.saturating_sub(raw_image.bit_depth);
    if shift > 0 {
        println!(
            "Scaling {} bit data to 16-bit (shift {})",
            raw_image.bit_depth, shift
        );
        for pixel in &mut raw_image.data {
            *pixel <<= shift;
        }
    }

    println!("Demosaicing with Bilinear interpolation...");
    let demosaic = Bilinear;
    let mut rgb_image = demosaic.demosaic(&raw_image);

    // 3. White Balance
    // These coefficients are approximate for "Daylight" on typical Sony sensors.
    // In a real raw converter, these would come from the "as shot" metadata or auto-WB analysis.
    println!("Applying White Balance (Daylight)...");
    apply_white_balance(&mut rgb_image, (2.35, 1.0, 1.65));

    // 4. Color Matrix (Camera to sRGB)
    // This is a generic "Neutral" matrix that boosts saturation slightly to look pleasing.
    println!("Applying Color Matrix...");
    #[rustfmt::skip]
    let matrix = [
        1.6, -0.4, -0.2,
        -0.2, 1.4, -0.2,
        -0.1, -0.3, 1.4,
    ];
    apply_color_matrix(&mut rgb_image, &matrix);

    // 5. Gamma Correction
    println!("Applying Gamma Correction (2.2)...");
    apply_gamma(&mut rgb_image, 2.2);

    println!("Saving to PNG: {}", output_path);

    let mut u8_data = Vec::with_capacity(rgb_image.data.len() * 2);
    for &pixel in &rgb_image.data {
        let bytes = pixel.to_be_bytes();
        u8_data.push(bytes[0]);
        u8_data.push(bytes[1]);
    }

    let options = EncoderOptions::default()
        .set_width(rgb_image.width as usize)
        .set_height(rgb_image.height as usize)
        .set_colorspace(ColorSpace::RGB)
        .set_depth(BitDepth::Sixteen);

    let mut encoder = PngEncoder::new(&u8_data, options);
    let png_data = encoder.encode();

    std::fs::write(output_path, png_data)?;
    println!("Done!");

    Ok(())
}

// TODO: Refine API for this usage more
