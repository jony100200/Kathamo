//! Presents CPU Safe-Mode frames into an OS window without any GPU API
//! (GDI on Windows, X11/Wayland shared memory on Linux) via `softbuffer`.

use crate::gpu_backend::SharedWindow;
use std::num::NonZeroU32;

pub struct SoftwarePresenter {
    surface: softbuffer::Surface<SharedWindow, SharedWindow>,
    _context: softbuffer::Context<SharedWindow>,
    size: (u32, u32),
}

impl SoftwarePresenter {
    pub fn new(window: SharedWindow) -> Result<Self, String> {
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| format!("softbuffer context: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window)
            .map_err(|e| format!("softbuffer surface: {e}"))?;
        Ok(Self {
            surface,
            _context: context,
            size: (0, 0),
        })
    }

    /// Copies 0xAARRGGBB pixels to the window and presents them.
    pub fn present(&mut self, pixels: &[u32], width: u32, height: u32) -> Result<(), String> {
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return Ok(());
        };
        if pixels.len() < (width * height) as usize {
            return Err("pixel buffer smaller than presentation size".into());
        }
        if self.size != (width, height) {
            self.surface
                .resize(w, h)
                .map_err(|e| format!("softbuffer resize: {e}"))?;
            self.size = (width, height);
        }
        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|e| format!("softbuffer buffer: {e}"))?;
        for (dst, src) in buffer.iter_mut().zip(pixels.iter()) {
            *dst = src & 0x00FF_FFFF; // softbuffer requires the top byte to be zero
        }
        buffer
            .present()
            .map_err(|e| format!("softbuffer present: {e}"))
    }
}
