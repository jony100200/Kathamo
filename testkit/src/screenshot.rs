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
}
