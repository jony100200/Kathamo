//! Native offscreen frame buffer capture for visual QA matrix tests.
//! Supports dumping pixel buffers to PPM/raw image formats without browser or OS desktop dependencies.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct ScreenshotCapture;

impl ScreenshotCapture {
    /// Saves an RGBA/ARGB pixel buffer as a standard portable pixmap (PPM).
    pub fn save_ppm(
        path: impl AsRef<Path>,
        pixels: &[u32],
        width: u32,
        height: u32,
    ) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Header: P6, width, height, maxval 255
        writeln!(writer, "P6")?;
        writeln!(writer, "{} {}", width, height)?;
        writeln!(writer, "255")?;

        for &pixel in pixels {
            let r = ((pixel >> 16) & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let b = (pixel & 0xFF) as u8;
            writer.write_all(&[r, g, b])?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Saves an ARGB pixel buffer as an RGB PNG that any image viewer opens.
    /// Uses uncompressed (stored) deflate blocks, so no compression crate is needed.
    pub fn save_png(
        path: impl AsRef<Path>,
        pixels: &[u32],
        width: u32,
        height: u32,
    ) -> std::io::Result<()> {
        std::fs::write(path, encode_png(pixels, width, height))
    }
}

/// Encodes ARGB pixels as an 8-bit RGB PNG.
pub fn encode_png(pixels: &[u32], width: u32, height: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((height * (width * 3 + 1)) as usize);
    for row in pixels.chunks(width as usize).take(height as usize) {
        raw.push(0); // filter type: none
        for &p in row {
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }

    // zlib stream: header, stored blocks of at most 65535 bytes, Adler-32.
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        let len = block.len() as u16;
        z.push(u8::from(i + 1 == blocks.len()));
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, RGB, deflate, no filter, no interlace

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    write_chunk(&mut png, b"IHDR", &ihdr);
    write_chunk(&mut png, b"IDAT", &z);
    write_chunk(&mut png, b"IEND", &[]);
    png
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in kind.iter().chain(data) {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_capture_writes_valid_ppm() {
        let pixels = vec![0xFFFF0000; 16]; // 4x4 red image
        let path = std::env::temp_dir().join("test_ui_capture.ppm");
        let res = ScreenshotCapture::save_ppm(&path, &pixels, 4, 4);
        assert!(res.is_ok());
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn png_encoder_matches_known_crc_and_layout() {
        let png = encode_png(&[0xFFFF0000, 0xFF00FF00], 2, 1);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR CRC for a 2x1 8-bit RGB image, as computed by Python's zlib.crc32.
        assert_eq!(&png[29..33], &[0x7B, 0x40, 0xE8, 0xDD]);
        assert_eq!(&png[png.len() - 12..], b"\0\0\0\0IEND\xAE\x42\x60\x82");
    }
}
