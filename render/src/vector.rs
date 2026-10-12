//! SVG icon pipeline: Tabler SVG source -> `usvg` parse (once) -> `resvg` raster
//! (once per pixel size) -> 8-bit coverage mask recolored by theme at draw time.
//!
//! Icons are monochrome stroke icons (`stroke="currentColor"`), so the rendered alpha
//! channel is used as a coverage mask and tinted with the requested theme colour.

use crate::text::Mask;
use resvg::{tiny_skia, usvg};
use std::collections::HashMap;
use std::sync::Arc;

/// Parses and rasterizes an SVG document into a `width` x `height` coverage mask.
pub fn rasterize_svg(tree: &usvg::Tree, width: u32, height: u32) -> Option<Mask> {
    if width == 0 || height == 0 {
        return None;
    }
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width().max(1.0),
        height as f32 / size.height().max(1.0),
    );
    resvg::render(tree, transform, &mut pixmap.as_mut());
    let data = pixmap.pixels().iter().map(|p| p.alpha()).collect();
    Some(Mask {
        width,
        height,
        left: 0,
        top: 0,
        data,
    })
}

/// Parses an SVG document. Returns `None` for malformed input.
pub fn parse_svg(svg: &str) -> Option<usvg::Tree> {
    usvg::Tree::from_str(svg, &usvg::Options::default()).ok()
}

/// Key for a rasterized icon: icon identifier + physical pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconKey {
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
}

/// Two-level icon cache: parsed vector trees per icon, and raster masks per size.
#[derive(Default)]
pub struct IconRasterCache {
    trees: HashMap<&'static str, Option<Arc<usvg::Tree>>>,
    masks: HashMap<IconKey, Option<Arc<Mask>>>,
}

impl IconRasterCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&mut self, key: IconKey, svg: &'static str) -> Option<Arc<Mask>> {
        if let Some(hit) = self.masks.get(&key) {
            return hit.clone();
        }
        let tree = self
            .trees
            .entry(key.id)
            .or_insert_with(|| parse_svg(svg).map(Arc::new))
            .clone();
        let mask = tree
            .and_then(|t| rasterize_svg(&t, key.width, key.height))
            .map(Arc::new);
        self.masks.insert(key, mask.clone());
        mask
    }

    pub fn parsed_count(&self) -> usize {
        self.trees.len()
    }

    pub fn raster_count(&self) -> usize {
        self.masks.len()
    }

    pub fn clear_rasters(&mut self) {
        self.masks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M5 12l5 5l10 -10" /></svg>"#;

    #[test]
    fn tabler_svg_rasterizes_to_antialiased_mask() {
        let tree = parse_svg(CHECK).expect("valid svg");
        let mask = rasterize_svg(&tree, 32, 32).unwrap();
        let covered = mask.data.iter().filter(|&&a| a > 0).count();
        let partial = mask.data.iter().filter(|&&a| a > 0 && a < 255).count();
        assert!(covered > 40, "stroke must produce coverage");
        assert!(partial > 0, "edges must be anti-aliased");
        // The invisible bounding path must not fill the square.
        assert!(covered < 32 * 32 / 2);
    }

    #[test]
    fn icon_cache_parses_once_and_rasterizes_per_size() {
        let mut cache = IconRasterCache::new();
        for _ in 0..3 {
            cache.get(
                IconKey {
                    id: "check",
                    width: 16,
                    height: 16,
                },
                CHECK,
            );
        }
        cache.get(
            IconKey {
                id: "check",
                width: 32,
                height: 32,
            },
            CHECK,
        );
        assert_eq!(cache.parsed_count(), 1);
        assert_eq!(cache.raster_count(), 2);
    }
}
