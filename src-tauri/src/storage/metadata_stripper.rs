// src-tauri/src/storage/metadata_stripper.rs
// Lossless privacy metadata stripping for JPEG (EXIF/XMP/APPn/COM) and PNG (eXIf/tEXt/zTXt/iTXt/tIME).

use std::path::Path;

/// Strip privacy-sensitive metadata (GPS, camera serial, author, software, timestamps)
/// from JPEG and PNG payloads while preserving pixel data losslessly.
/// Returns the stripped bytes if the file is a supported JPEG/PNG with metadata removed,
/// or the original bytes unchanged if no metadata was present, unsupported, or malformed.
pub fn strip_metadata_if_image(file_name_or_path: &Path, data: &[u8]) -> Vec<u8> {
    let ext = file_name_or_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" => strip_jpeg_metadata(data).unwrap_or_else(|| data.to_vec()),
        "png" => strip_png_metadata(data).unwrap_or_else(|| data.to_vec()),
        _ => data.to_vec(),
    }
}

/// Strip APP1..APP15 (EXIF, XMP, IPTC, Adobe) and COM segments from a JPEG stream.
/// Keeps SOI, APP0 (JFIF), SOF*, DHT, DQT, DRI, SOS + entropy-coded scan data.
/// Returns `None` if the input is not a valid JPEG or appears truncated.
pub fn strip_jpeg_metadata(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[0..2]); // SOI
    let mut i = 2;

    while i < data.len() {
        // Skip any fill 0xFF bytes before marker
        while i < data.len() && data[i] != 0xFF {
            return None;
        }
        while i + 1 < data.len() && data[i + 1] == 0xFF {
            i += 1;
        }
        if i + 1 >= data.len() {
            return None;
        }

        let marker = data[i + 1];

        // Standalone markers (no length field)
        if marker == 0x00 || marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            out.extend_from_slice(&data[i..i + 2]);
            i += 2;
            continue;
        }

        // EOI (End of Image)
        if marker == 0xD9 {
            out.extend_from_slice(&data[i..i + 2]);
            return Some(out);
        }

        // SOS (Start of Scan): copy SOS header + all remaining entropy-coded scan bytes verbatim
        if marker == 0xDA {
            out.extend_from_slice(&data[i..]);
            return Some(out);
        }

        // Segment with 2-byte big-endian length (length includes the 2 length bytes)
        if i + 4 > data.len() {
            return None;
        }
        let seg_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if seg_len < 2 || i + 2 + seg_len > data.len() {
            return None;
        }

        // Strip APP1..APP15 (0xE1..=0xEF) and COM (0xFE)
        let should_strip = (0xE1..=0xEF).contains(&marker) || marker == 0xFE;
        if !should_strip {
            out.extend_from_slice(&data[i..i + 2 + seg_len]);
        }

        i += 2 + seg_len;
    }

    Some(out)
}

/// Strip ancillary privacy-sensitive chunks (`eXIf`, `tEXt`, `zTXt`, `iTXt`, `tIME`) from a PNG.
/// Returns `None` if the input is not a valid PNG or appears truncated.
pub fn strip_png_metadata(data: &[u8]) -> Option<Vec<u8>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if data.len() < 8 || &data[0..8] != PNG_SIG {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(PNG_SIG);
    let mut i = 8;
    let mut saw_iend = false;

    while i < data.len() {
        if i + 12 > data.len() {
            return None;
        }
        let chunk_len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let total_chunk_bytes = 4usize.checked_add(4)?.checked_add(chunk_len)?.checked_add(4)?;
        if i.checked_add(total_chunk_bytes)? > data.len() {
            return None;
        }

        let chunk_type = &data[i + 4..i + 8];
        let should_strip = matches!(
            chunk_type,
            b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"tIME"
        );

        if !should_strip {
            out.extend_from_slice(&data[i..i + total_chunk_bytes]);
        }

        if chunk_type == b"IEND" {
            saw_iend = true;
            break;
        }

        i += total_chunk_bytes;
    }

    if saw_iend {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jpeg_exif_and_com_preserves_image_data() {
        let mut jpeg = Vec::new();
        // SOI
        jpeg.extend_from_slice(&[0xFF, 0xD8]);
        // APP0 (JFIF) - should be kept
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x06, b'J', b'F', b'I', b'F']);
        // APP1 (EXIF with GPS) - should be stripped
        jpeg.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x08, b'E', b'x', b'i', b'f', 0x00, 0x00]);
        // COM (Comment) - should be stripped
        jpeg.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x06, b'u', b's', b'e', b'r']);
        // SOS + dummy scan + EOI - should be kept verbatim
        jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x04, 0x01, 0x00, 0xAA, 0xBB, 0xFF, 0xD9]);

        let stripped = strip_metadata_if_image(Path::new("lecture_board.jpg"), &jpeg);
        assert!(stripped.len() < jpeg.len());
        // Must still start with SOI and contain APP0 + SOS
        assert_eq!(&stripped[0..2], &[0xFF, 0xD8]);
        assert!(stripped.windows(4).any(|w| w == b"JFIF"));
        assert!(!stripped.windows(4).any(|w| w == b"Exif"));
        assert!(!stripped.windows(4).any(|w| w == b"user"));
        assert!(stripped.ends_with(&[0xAA, 0xBB, 0xFF, 0xD9]));
    }

    #[test]
    fn test_strip_png_exif_and_text_chunks_preserves_idat() {
        fn make_chunk(ctype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut c = Vec::new();
            c.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            c.extend_from_slice(ctype);
            c.extend_from_slice(payload);
            c.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // dummy CRC
            c
        }

        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&make_chunk(b"IHDR", &[1, 2, 3, 4]));
        png.extend_from_slice(&make_chunk(b"tEXt", b"Author\0StudentName"));
        png.extend_from_slice(&make_chunk(b"eXIf", b"GPSCoordsHere"));
        png.extend_from_slice(&make_chunk(b"iTXt", b"XML:com.adobe.xmp"));
        png.extend_from_slice(&make_chunk(b"IDAT", b"PIXELDATA"));
        png.extend_from_slice(&make_chunk(b"IEND", b""));

        let stripped = strip_metadata_if_image(Path::new("diagram.png"), &png);
        assert!(stripped.windows(4).any(|w| w == b"IHDR"));
        assert!(stripped.windows(9).any(|w| w == b"PIXELDATA"));
        assert!(stripped.windows(4).any(|w| w == b"IEND"));
        assert!(!stripped.windows(11).any(|w| w == b"StudentName"));
        assert!(!stripped.windows(13).any(|w| w == b"GPSCoordsHere"));
        assert!(!stripped.windows(4).any(|w| w == b"iTXt"));
    }

    #[test]
    fn test_malformed_or_non_image_returns_original() {
        let malformed = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0xFF]; // truncated length
        assert_eq!(strip_metadata_if_image(Path::new("broken.jpg"), &malformed), malformed);

        let pdf = b"%PDF-1.7\n";
        assert_eq!(strip_metadata_if_image(Path::new("notes.pdf"), pdf), pdf);
    }
}
