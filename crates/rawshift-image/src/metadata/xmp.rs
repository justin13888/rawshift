//! XMP metadata embedding for image export.
//!
//! Provides XMP embedding functions for the JPEG and AVIF containers (PNG and
//! JXL embed XMP through their gamut encoders instead). Every payload is
//! validated with `gamut-xmp` before it is embedded, so a malformed packet is
//! rejected instead of being spliced into the output.

/// Error type for XMP embedding operations.
#[derive(Debug)]
pub enum XmpError {
    /// The XMP payload is not a well-formed XMP packet
    Invalid(String),
    /// Failed to manipulate image container
    Container(String),
}

impl std::fmt::Display for XmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmpError::Invalid(msg) => write!(f, "Invalid XMP packet: {}", msg),
            XmpError::Container(msg) => write!(f, "XMP container error: {}", msg),
        }
    }
}

impl std::error::Error for XmpError {}

impl From<img_parts::Error> for XmpError {
    fn from(e: img_parts::Error) -> Self {
        XmpError::Container(e.to_string())
    }
}

impl From<std::io::Error> for XmpError {
    fn from(e: std::io::Error) -> Self {
        XmpError::Container(e.to_string())
    }
}

/// Adobe XMP namespace marker used in JPEG APP1 segments.
const XMP_JPEG_NS: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// Validate an XMP payload with `gamut-xmp` before embedding it.
///
/// Rejecting malformed packets here keeps garbage out of the output
/// containers; a payload that round-trips through
/// [`gamut_xmp::XmpMeta::from_packet`] is embeddable.
fn validate_xmp(xmp_bytes: &[u8]) -> Result<(), XmpError> {
    gamut_xmp::XmpMeta::from_packet(xmp_bytes)
        .map(|_| ())
        .map_err(|e| XmpError::Invalid(e.to_string()))
}

/// Append XMP metadata to existing JPEG data.
///
/// XMP is embedded as an APP1 segment (0xE1) with the Adobe XMP namespace prefix
/// `http://ns.adobe.com/xap/1.0/\0` followed by the raw XMP packet bytes.
/// The payload is validated with `gamut-xmp` first.
pub fn append_xmp_to_jpeg(xmp_bytes: &[u8], jpeg_data: Vec<u8>) -> Result<Vec<u8>, XmpError> {
    use img_parts::Bytes;
    use img_parts::jpeg::{Jpeg, JpegSegment, markers};
    use std::io::Cursor;

    validate_xmp(xmp_bytes)?;

    let mut contents = Vec::with_capacity(XMP_JPEG_NS.len() + xmp_bytes.len());
    contents.extend_from_slice(XMP_JPEG_NS);
    contents.extend_from_slice(xmp_bytes);

    let mut jpeg = Jpeg::from_bytes(Bytes::from(jpeg_data))?;
    let xmp_segment = JpegSegment::new_with_contents(markers::APP1, Bytes::from(contents));

    // Insert just before the first segment with entropy (SOS), or at the end.
    let pos = jpeg
        .segments()
        .iter()
        .position(|s| s.has_entropy())
        .unwrap_or_else(|| jpeg.segments().len());
    jpeg.segments_mut().insert(pos, xmp_segment);

    let mut output = Cursor::new(Vec::new());
    jpeg.encoder().write_to(&mut output)?;
    Ok(output.into_inner())
}

/// Append XMP metadata to an in-memory AVIF byte stream.
///
/// A top-level `xml ` ISOBMFF box containing the XMP packet is appended to the
/// end of the data.  Since the new box follows all existing boxes (including
/// `mdat`), no `iloc` extent offsets need to be patched.
/// The payload is validated with `gamut-xmp` first.
#[cfg_attr(not(feature = "avif"), allow(dead_code))]
pub fn append_xmp_to_avif(xmp_bytes: &[u8], avif_data: Vec<u8>) -> Result<Vec<u8>, XmpError> {
    validate_xmp(xmp_bytes)?;

    let mut data = avif_data;
    let box_size = (8 + xmp_bytes.len()) as u32;
    data.reserve(box_size as usize);
    data.extend_from_slice(&box_size.to_be_bytes());
    data.extend_from_slice(b"xml ");
    data.extend_from_slice(xmp_bytes);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal well-formed XMP body (`rdf:RDF` is required by the validator).
    const VALID_XMP: &[u8] = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
        <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>\
        </x:xmpmeta>";

    #[test]
    fn test_append_xmp_to_jpeg_basic() {
        // Build a minimal valid JPEG: SOI + APP0 (JFIF) + EOI
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8]); // SOI
        // APP0 (JFIF marker)
        jpeg.extend_from_slice(&[0xFF, 0xE0]);
        let app0_len: u16 = 16;
        jpeg.extend_from_slice(&app0_len.to_be_bytes());
        jpeg.extend_from_slice(b"JFIF\0");
        jpeg.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]); // JFIF header rest
        jpeg.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let xmp = VALID_XMP;
        let result = append_xmp_to_jpeg(xmp, jpeg).expect("XMP embed should succeed");

        // Must still be a valid JPEG (starts with SOI)
        assert_eq!(&result[0..2], &[0xFF, 0xD8]);
        // Must contain the XMP namespace prefix
        let has_ns = result.windows(XMP_JPEG_NS.len()).any(|w| w == XMP_JPEG_NS);
        assert!(has_ns, "output must contain XMP namespace marker");
        // Must contain the XMP payload
        let has_xmp = result.windows(xmp.len()).any(|w| w == xmp);
        assert!(has_xmp, "output must contain XMP payload");
    }

    #[test]
    fn test_malformed_xmp_is_rejected() {
        // Not XML at all → every embed path must refuse to splice it in.
        let avif = vec![0u8; 32];
        assert!(matches!(
            append_xmp_to_avif(b"not xmp", avif),
            Err(XmpError::Invalid(_))
        ));
        // XML without an rdf:RDF element is not an XMP packet either.
        assert!(matches!(
            append_xmp_to_avif(b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>", vec![0u8; 8]),
            Err(XmpError::Invalid(_))
        ));
    }

    #[test]
    fn test_append_xmp_to_avif() {
        // `append_xmp_to_avif` only appends a trailing `xml ` box, so the
        // leading bytes need not form a real container for this unit test.
        let avif = vec![0u8; 32];
        let xmp = VALID_XMP;
        let result = append_xmp_to_avif(xmp, avif).expect("AVIF XMP embed should succeed");

        let box_size = (8 + xmp.len()) as u32;
        assert_eq!(result.len(), 32 + box_size as usize);
        assert_eq!(&result[32..36], &box_size.to_be_bytes());
        assert_eq!(&result[36..40], b"xml ");
        assert_eq!(&result[40..], xmp);
    }
}
