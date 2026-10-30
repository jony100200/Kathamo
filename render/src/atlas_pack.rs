//! Glyph/icon atlas packing: slot records and a shelf packer for the R8 atlas texture.

use crate::text::GlyphKey;
use crate::vector::IconKey;

pub(crate) const ATLAS_SIZE: u32 = 2048;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Slot {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) left: i32,
    pub(crate) top: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AtlasKey {
    Glyph(GlyphKey),
    Icon(IconKey),
}

/// Simple shelf packer for the glyph/icon atlas.
#[derive(Debug, Default)]
pub(crate) struct ShelfPacker {
    x: u32,
    y: u32,
    row_h: u32,
}

impl ShelfPacker {
    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let (w, h) = (w + 1, h + 1); // 1px gutter against sampling bleed
        if w > ATLAS_SIZE || h > ATLAS_SIZE {
            return None;
        }
        if self.x + w > ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_h;
            self.row_h = 0;
        }
        if self.y + h > ATLAS_SIZE {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += w;
        self.row_h = self.row_h.max(h);
        Some(pos)
    }
}
